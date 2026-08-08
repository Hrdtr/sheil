use std::collections::HashMap;
use std::sync::Arc;

use russh::client;
use russh::{Channel, ChannelMsg};
use serde::Serialize;
use tauri::{command, AppHandle, Manager};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::sync::Mutex;

use crate::commands::ssh::SshState;
use crate::db;

// ---------------------------------------------------------------------------
// Public types (serializable, used in commands)
// ---------------------------------------------------------------------------

/// Unique identifier for an active forwarding tunnel.
type ForwardId = String;

/// Describes the tunnel type for the UI.
#[derive(Serialize, Clone)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum ForwardKind {
    Local {
        local_addr: String,
        local_port: u16,
        remote_host: String,
        remote_port: u16,
    },
    Remote {
        remote_listen_addr: String,
        remote_listen_port: u16,
        target_host: String,
        target_port: u16,
    },
    Dynamic {
        local_addr: String,
        local_port: u16,
    },
}

/// Snapshot of an active tunnel sent to the frontend.
#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ForwardInfo {
    pub id: ForwardId,
    pub session_id: String,
    pub kind: ForwardKind,
    pub label: String,
}

// ---------------------------------------------------------------------------
// Desktop forwarding state
// ---------------------------------------------------------------------------

/// Payload delivered from the `Client` handler when an incoming
/// forwarded-tcpip channel arrives (remote forwarding).
pub struct RemoteForwardEvent {
    pub channel: Channel<client::Msg>,
    pub connected_address: String,
    pub connected_port: u32,
    pub originator_address: String,
    pub originator_port: u32,
    pub session_id: String,
}

#[cfg(not(any(target_os = "ios", target_os = "android")))]
struct ForwardEntry {
    info: ForwardInfo,
    /// Aborts the accept-loop task for local/dynamic forwarding.
    accept_abort: Option<tokio::task::AbortHandle>,
    /// Abort handles for active per-connection bridge tasks.
    bridge_aborts: Vec<tokio::task::AbortHandle>,
}

#[cfg(not(any(target_os = "ios", target_os = "android")))]
struct ForwardingStateInner {
    tunnels: Mutex<HashMap<ForwardId, ForwardEntry>>,
    remote_forward_tx: tokio::sync::mpsc::Sender<RemoteForwardEvent>,
}

/// Holds all active port forwarding tunnels. Managed by Tauri alongside
/// `SshState`.
///
/// On mobile this struct is empty (all functionality is gated).
pub struct ForwardingState {
    #[cfg(not(any(target_os = "ios", target_os = "android")))]
    inner: Arc<ForwardingStateInner>,
}

#[cfg(not(any(target_os = "ios", target_os = "android")))]
impl ForwardingState {
    pub fn new(_app_handle: AppHandle) -> (Self, tokio::sync::mpsc::Sender<RemoteForwardEvent>) {
        let (remote_forward_tx, remote_forward_rx) = tokio::sync::mpsc::channel(64);

        let inner = Arc::new(ForwardingStateInner {
            tunnels: Mutex::new(HashMap::new()),
            remote_forward_tx: remote_forward_tx.clone(),
        });

        // Background task: drains incoming forwarded-tcpip channels (remote forwarding).
        // Use block_on since we're called from synchronous setup() and need a runtime.
        let inner_clone = inner.clone();
        tauri::async_runtime::block_on(async {
            tokio::spawn(async move {
                drain_remote_forwards(inner_clone, remote_forward_rx).await;
            });
        });

        (Self { inner }, remote_forward_tx)
    }

    pub fn remote_forward_sender(&self) -> tokio::sync::mpsc::Sender<RemoteForwardEvent> {
        self.inner.remote_forward_tx.clone()
    }

    async fn insert_entry(&self, entry: ForwardEntry) {
        self.inner
            .tunnels
            .lock()
            .await
            .insert(entry.info.id.clone(), entry);
    }

    async fn get_entry(&self, id: &str) -> Option<ForwardEntry> {
        let tunnels = self.inner.tunnels.lock().await;
        tunnels.get(id).map(|e| ForwardEntry {
            info: e.info.clone(),
            accept_abort: e.accept_abort.clone(),
            bridge_aborts: e.bridge_aborts.clone(),
        })
    }

    async fn remove_and_abort(&self, id: &str) -> Option<ForwardInfo> {
        let mut tunnels = self.inner.tunnels.lock().await;
        let entry = tunnels.remove(id)?;
        if let Some(abort) = entry.accept_abort {
            abort.abort();
        }
        for abort in &entry.bridge_aborts {
            abort.abort();
        }
        Some(entry.info)
    }

    async fn list_infos(&self, session_id: Option<&str>) -> Vec<ForwardInfo> {
        let tunnels = self.inner.tunnels.lock().await;
        tunnels
            .values()
            .filter(|e| session_id.map_or(true, |sid| e.info.session_id == sid))
            .map(|e| e.info.clone())
            .collect()
    }

    pub async fn stop_all_for_session(&self, session_id: &str) {
        let ids: Vec<String> = {
            let tunnels = self.inner.tunnels.lock().await;
            tunnels
                .iter()
                .filter(|(_, e)| e.info.session_id == session_id)
                .map(|(id, _)| id.clone())
                .collect()
        };
        for id in ids {
            self.remove_and_abort(&id).await;
        }
    }
}

#[cfg(any(target_os = "ios", target_os = "android"))]
pub struct ForwardingState;

#[cfg(any(target_os = "ios", target_os = "android"))]
impl ForwardingState {
    pub fn new(_app_handle: AppHandle) -> Self {
        Self
    }
}

// ---------------------------------------------------------------------------
// Core bridge task
// ---------------------------------------------------------------------------

#[cfg(not(any(target_os = "ios", target_os = "android")))]
use tokio::net::TcpStream;

/// Bidirectional data pipe between a `TcpStream` and an SSH channel.
///
/// Spawns a single tokio task using `tokio::select!` for both directions.
/// Returns an `AbortHandle` for cancellation.
#[cfg(not(any(target_os = "ios", target_os = "android")))]
fn spawn_bridge(tcp: TcpStream, channel: Channel<client::Msg>) -> tokio::task::AbortHandle {
    tokio::spawn(async move {
        bridge_tcp_channel(tcp, channel).await;
    })
    .abort_handle()
}

#[cfg(not(any(target_os = "ios", target_os = "android")))]
async fn bridge_tcp_channel(tcp: TcpStream, channel: Channel<client::Msg>) {
    let (mut chan_read, chan_write) = channel.split();
    let (mut tcp_read, mut tcp_write) = tcp.into_split();

    let chan_write = Arc::new(Mutex::new(chan_write));

    let tcp_to_chan = {
        let chan_write = Arc::clone(&chan_write);
        async move {
            let mut buf = vec![0u8; 16384];
            loop {
                let n = match tcp_read.read(&mut buf).await {
                    Ok(0) => break,
                    Ok(n) => n,
                    Err(e) => {
                        log::debug!("bridge: tcp read error: {e}");
                        break;
                    }
                };
                let write = chan_write.lock().await;
                if write.data(&buf[..n]).await.is_err() {
                    break;
                }
            }
            let write = chan_write.lock().await;
            let _ = write.eof().await;
        }
    };

    let chan_to_tcp = async move {
        loop {
            match chan_read.wait().await {
                Some(ChannelMsg::Data { data }) => {
                    if tcp_write.write_all(&data).await.is_err() {
                        break;
                    }
                }
                Some(ChannelMsg::Eof) | None => break,
                Some(_) => {}
            }
        }
        let _ = tcp_write.shutdown().await;
    };

    tokio::select! {
        () = tcp_to_chan => {},
        () = chan_to_tcp => {},
    }
}

// ---------------------------------------------------------------------------
// Background task — remote forward channel dispatch
// ---------------------------------------------------------------------------

#[cfg(not(any(target_os = "ios", target_os = "android")))]
async fn drain_remote_forwards(
    state: Arc<ForwardingStateInner>,
    mut rx: tokio::sync::mpsc::Receiver<RemoteForwardEvent>,
) {
    while let Some(event) = rx.recv().await {
        log::debug!(
            "remote forward: connection from {}:{} (origin: {}:{})",
            event.connected_address,
            event.connected_port,
            event.originator_address,
            event.originator_port
        );

        // Find matching forward config
        let target: Option<(String, u16)> = {
            let tunnels = state.tunnels.lock().await;
            tunnels
                .values()
                .find(|e| {
                    e.info.session_id == event.session_id
                        && matches!(&e.info.kind, ForwardKind::Remote {
                            remote_listen_port, ..
                        } if u32::from(*remote_listen_port) == event.connected_port)
                })
                .map(|e| match &e.info.kind {
                    ForwardKind::Remote {
                        target_host,
                        target_port,
                        ..
                    } => (target_host.clone(), *target_port),
                    _ => unreachable!(),
                })
        };

        if let Some((host, port)) = target {
            log::info!("remote forward: connecting to {host}:{port}");
            match TcpStream::connect((host.as_str(), port)).await {
                Ok(tcp) => {
                    let abort = spawn_bridge(tcp, event.channel);
                    let mut tunnels = state.tunnels.lock().await;
                    // Store abort handle on the matching forward entry
                    for entry in tunnels.values_mut() {
                        if entry.info.session_id == event.session_id
                            && matches!(entry.info.kind, ForwardKind::Remote { .. })
                        {
                            entry.bridge_aborts.push(abort);
                            break;
                        }
                    }
                }
                Err(e) => {
                    log::error!("remote forward: failed to connect to {host}:{port}: {e}");
                    let (_, write) = event.channel.split();
                    let _ = write.eof().await;
                }
            }
        } else {
            log::warn!(
                "remote forward: no matching forward for port {} on session {}",
                event.connected_port,
                event.session_id
            );
            let (_, write) = event.channel.split();
            let _ = write.eof().await;
        }
    }
    log::info!("remote forward drain task exited");
}

// ---------------------------------------------------------------------------
// Tauri commands
// ---------------------------------------------------------------------------

/// Start a local port forwarding tunnel.
#[command]
#[allow(clippy::too_many_arguments)]
pub async fn port_forward_start_local(
    app_handle: AppHandle,
    fwd_state: tauri::State<'_, ForwardingState>,
    session_id: String,
    local_addr: String,
    local_port: u16,
    remote_host: String,
    remote_port: u16,
) -> Result<ForwardInfo, String> {
    #[cfg(not(any(target_os = "ios", target_os = "android")))]
    {
        return start_local_impl(
            app_handle,
            fwd_state,
            session_id,
            local_addr,
            local_port,
            remote_host,
            remote_port,
        )
        .await;
    }
    #[cfg(any(target_os = "ios", target_os = "android"))]
    {
        let _ = (
            app_handle,
            fwd_state,
            session_id,
            local_addr,
            local_port,
            remote_host,
            remote_port,
        );
        Err("port forwarding not available on mobile".into())
    }
}

#[cfg(not(any(target_os = "ios", target_os = "android")))]
async fn start_local_impl(
    app_handle: AppHandle,
    fwd_state: tauri::State<'_, ForwardingState>,
    session_id: String,
    local_addr: String,
    local_port: u16,
    remote_host: String,
    remote_port: u16,
) -> Result<ForwardInfo, String> {
    let addr = format!("{local_addr}:{local_port}");

    // Check for duplicate
    {
        let infos = fwd_state.list_infos(Some(&session_id)).await;
        if infos.iter().any(
            |i| matches!(&i.kind, ForwardKind::Local { local_port: p, .. } if *p == local_port),
        ) {
            return Err(ForwardError::DuplicateForward(local_addr, local_port).into());
        }
    }

    let listener = tokio::net::TcpListener::bind(&addr)
        .await
        .map_err(|e| ForwardError::Io(e).to_string())?;

    let forward_id = db::new_table_row_id();
    let info = ForwardInfo {
        id: forward_id.clone(),
        session_id: session_id.clone(),
        kind: ForwardKind::Local {
            local_addr: local_addr.clone(),
            local_port,
            remote_host: remote_host.clone(),
            remote_port,
        },
        label: format!("{local_addr}:{local_port} → {remote_host}:{remote_port}"),
    };

    let app = app_handle.clone();
    let sid = session_id.clone();
    let rhost = remote_host.clone();
    let accept_abort = tokio::spawn(async move {
        accept_loop_local(app, listener, sid, rhost, remote_port).await;
    })
    .abort_handle();

    fwd_state
        .insert_entry(ForwardEntry {
            info: info.clone(),
            accept_abort: Some(accept_abort),
            bridge_aborts: Vec::new(),
        })
        .await;

    log::info!("local forward {forward_id} started: {local_addr}:{local_port} → {remote_host}:{remote_port}");
    Ok(info)
}

#[cfg(not(any(target_os = "ios", target_os = "android")))]
async fn accept_loop_local(
    app_handle: AppHandle,
    listener: tokio::net::TcpListener,
    session_id: String,
    remote_host: String,
    remote_port: u16,
) {
    loop {
        let (tcp, peer_addr) = match listener.accept().await {
            Ok(conn) => conn,
            Err(e) => {
                log::debug!("local forward accept loop: {e}");
                break;
            }
        };
        log::debug!("local forward: accepted connection from {peer_addr}");

        let ssh_state = app_handle.state::<SshState>();
        match ssh_state
            .open_direct_tcpip_channel(
                &session_id,
                &remote_host,
                u32::from(remote_port),
                &peer_addr.ip().to_string(),
                u32::from(peer_addr.port()),
            )
            .await
        {
            Ok(channel) => {
                spawn_bridge(tcp, channel);
            }
            Err(e) => {
                log::error!("local forward: failed to open channel: {e}");
            }
        }
    }
    log::debug!("local forward accept loop exited for session {session_id}");
}

#[cfg(not(any(target_os = "ios", target_os = "android")))]
async fn accept_loop_dynamic(
    app_handle: AppHandle,
    listener: tokio::net::TcpListener,
    session_id: String,
) {
    loop {
        let (mut tcp, peer_addr) = match listener.accept().await {
            Ok(conn) => conn,
            Err(e) => {
                log::debug!("dynamic forward accept loop: {e}");
                break;
            }
        };
        log::debug!("dynamic forward: accepted connection from {peer_addr}");

        // Perform SOCKS5 handshake to discover the target
        let target = match socks5_handshake(&mut tcp).await {
            Ok((host, port)) => {
                log::debug!("SOCKS5: resolved target {host}:{port}");
                (host, port)
            }
            Err(e) => {
                log::debug!("SOCKS5 handshake failed from {peer_addr}: {e}");
                continue;
            }
        };

        // Open direct-tcpip channel through SSH
        let ssh_state = app_handle.state::<SshState>();
        match ssh_state
            .open_direct_tcpip_channel(
                &session_id,
                &target.0,
                u32::from(target.1),
                &peer_addr.ip().to_string(),
                u32::from(peer_addr.port()),
            )
            .await
        {
            Ok(channel) => {
                // Send SOCKS5 success reply
                let reply = socks5_reply(socks5_reply_code::SUCCEEDED, peer_addr);
                let _ = tcp.write_all(&reply).await;
                spawn_bridge(tcp, channel);
            }
            Err(e) => {
                log::error!(
                    "dynamic forward: failed to open channel to {}:{}: {}",
                    target.0,
                    target.1,
                    e
                );
                let reply = socks5_reply(socks5_reply_code::CONNECTION_REFUSED, peer_addr);
                let _ = tcp.write_all(&reply).await;
            }
        }
    }
    log::debug!("dynamic forward accept loop exited for session {session_id}");
}

// ---------------------------------------------------------------------------
// SOCKS5 protocol (RFC 1928)
// ---------------------------------------------------------------------------

/// SOCKS5 reply codes.
mod socks5_reply_code {
    pub const SUCCEEDED: u8 = 0x00;
    // pub const GENERAL_FAILURE: u8 = 0x01;
    // pub const CONNECTION_NOT_ALLOWED: u8 = 0x02;
    // pub const NETWORK_UNREACHABLE: u8 = 0x03;
    // pub const HOST_UNREACHABLE: u8 = 0x04;
    pub const CONNECTION_REFUSED: u8 = 0x05;
    // pub const TTL_EXPIRED: u8 = 0x06;
    pub const COMMAND_NOT_SUPPORTED: u8 = 0x07;
    pub const ADDRESS_TYPE_NOT_SUPPORTED: u8 = 0x08;
}

/// Build a SOCKS5 reply message.
#[cfg(not(any(target_os = "ios", target_os = "android")))]
fn socks5_reply(reply_code: u8, _bind_addr: std::net::SocketAddr) -> Vec<u8> {
    let mut reply = vec![0x05, reply_code, 0x00]; // version, reply, reserved
                                                  // BND.ADDR: use 0.0.0.0:0 since we're tunneling through SSH
    reply.extend_from_slice(&[0x01, 0, 0, 0, 0]); // IPv4, 0.0.0.0
    reply.extend_from_slice(&[0, 0]); // port 0
    reply
}

/// Perform the SOCKS5 CONNECT handshake (RFC 1928).
/// Returns the resolved target (host, port) on success.
#[cfg(not(any(target_os = "ios", target_os = "android")))]
async fn socks5_handshake(
    stream: &mut tokio::net::TcpStream,
) -> Result<(String, u16), ForwardError> {
    // Step 1: Read version + authentication methods
    let mut buf = [0u8; 2];
    read_exact(stream, &mut buf).await?;
    let version = buf[0];
    let nmethods = buf[1] as usize;

    if version != 0x05 {
        return Err(ForwardError::Socks5Protocol(format!(
            "unsupported SOCKS version: {version}"
        )));
    }

    // Read the list of methods (ignore them, we only support no-auth)
    let mut methods = vec![0u8; nmethods];
    read_exact(stream, &mut methods).await?;

    // Step 2: Reply with "no authentication required" (0x00)
    stream
        .write_all(&[0x05, 0x00])
        .await
        .map_err(ForwardError::Io)?;

    // Step 3: Read the client request
    // +----+-----+-------+------+----------+----------+
    // |VER | CMD |  RSV  | ATYP | DST.ADDR | DST.PORT |
    let mut req_header = [0u8; 4];
    read_exact(stream, &mut req_header).await?;
    let req_version = req_header[0];
    let command = req_header[1];
    // reserved = req_header[2];
    let atyp = req_header[3];

    if req_version != 0x05 {
        return Err(ForwardError::Socks5Protocol(
            "version mismatch in request".into(),
        ));
    }

    if command != 0x01 {
        // CONNECT only
        let reply_code = socks5_reply_code::COMMAND_NOT_SUPPORTED;
        let reply = build_reply(reply_code);
        let _ = stream.write_all(&reply).await;
        return Err(ForwardError::Socks5Protocol(format!(
            "unsupported command: {command}"
        )));
    }

    // Parse destination address
    if atyp != 0x01 && atyp != 0x03 && atyp != 0x04 {
        let reply = build_reply(socks5_reply_code::ADDRESS_TYPE_NOT_SUPPORTED);
        let _ = stream.write_all(&reply).await;
        return Err(ForwardError::Socks5Protocol(format!(
            "unsupported address type: {atyp}"
        )));
    }
    let host: String;
    match atyp {
        0x01 => {
            // IPv4
            let mut addr = [0u8; 4];
            read_exact(stream, &mut addr).await?;
            host = format!("{}.{}.{}.{}", addr[0], addr[1], addr[2], addr[3]);
        }
        0x03 => {
            // Domain name
            let mut len_buf = [0u8; 1];
            read_exact(stream, &mut len_buf).await?;
            let len = len_buf[0] as usize;
            let mut domain = vec![0u8; len];
            read_exact(stream, &mut domain).await?;
            host = String::from_utf8_lossy(&domain).to_string();
        }
        0x04 => {
            // IPv6
            let mut addr = [0u8; 16];
            read_exact(stream, &mut addr).await?;
            host = format!(
                "{:02x}{:02x}:{:02x}{:02x}:{:02x}{:02x}:{:02x}{:02x}:{:02x}{:02x}:{:02x}{:02x}:{:02x}{:02x}:{:02x}{:02x}",
                addr[0], addr[1], addr[2], addr[3],
                addr[4], addr[5], addr[6], addr[7],
                addr[8], addr[9], addr[10], addr[11],
                addr[12], addr[13], addr[14], addr[15],
            );
        }
        _ => unreachable!(),
    }

    // Read port (2 bytes, big-endian)
    let mut port_buf = [0u8; 2];
    read_exact(stream, &mut port_buf).await?;
    let port = u16::from_be_bytes(port_buf);

    Ok((host, port))
}

/// Read exactly `buf.len()` bytes from the stream, or return an error.
#[cfg(not(any(target_os = "ios", target_os = "android")))]
async fn read_exact(
    stream: &mut tokio::net::TcpStream,
    buf: &mut [u8],
) -> Result<(), ForwardError> {
    stream
        .read_exact(buf)
        .await
        .map(|_| ())
        .map_err(|e| ForwardError::Socks5Protocol(format!("read error: {e}")))
}

/// Build a SOCKS5 reply with BND.ADDR=0.0.0.0:0.
#[cfg(not(any(target_os = "ios", target_os = "android")))]
fn build_reply(reply_code: u8) -> Vec<u8> {
    let result = vec![0x05, reply_code, 0x00, 0x01, 0, 0, 0, 0, 0, 0];
    result
}

/// Start a remote port forwarding tunnel.
#[command]
#[allow(clippy::too_many_arguments)]
pub async fn port_forward_start_remote(
    app_handle: AppHandle,
    fwd_state: tauri::State<'_, ForwardingState>,
    session_id: String,
    listen_addr: String,
    listen_port: u16,
    target_host: String,
    target_port: u16,
) -> Result<ForwardInfo, String> {
    #[cfg(not(any(target_os = "ios", target_os = "android")))]
    {
        return start_remote_impl(
            app_handle,
            fwd_state,
            session_id,
            listen_addr,
            listen_port,
            target_host,
            target_port,
        )
        .await;
    }
    #[cfg(any(target_os = "ios", target_os = "android"))]
    {
        let _ = (
            app_handle,
            fwd_state,
            session_id,
            listen_addr,
            listen_port,
            target_host,
            target_port,
        );
        Err("port forwarding not available on mobile".into())
    }
}

#[cfg(not(any(target_os = "ios", target_os = "android")))]
async fn start_remote_impl(
    app_handle: AppHandle,
    fwd_state: tauri::State<'_, ForwardingState>,
    session_id: String,
    listen_addr: String,
    listen_port: u16,
    target_host: String,
    target_port: u16,
) -> Result<ForwardInfo, String> {
    let ssh_state = app_handle.state::<SshState>();

    let actual_port = ssh_state
        .tcpip_forward(&session_id, &listen_addr, u32::from(listen_port))
        .await
        .map_err(|e| ForwardError::Ssh(russh::Error::IO(std::io::Error::other(e))).to_string())?;

    let forward_id = db::new_table_row_id();
    let used_port = if actual_port > 0 {
        u16::try_from(actual_port).unwrap_or(u16::MAX)
    } else {
        listen_port
    };
    let info = ForwardInfo {
        id: forward_id.clone(),
        session_id: session_id.clone(),
        kind: ForwardKind::Remote {
            remote_listen_addr: listen_addr.clone(),
            remote_listen_port: used_port,
            target_host: target_host.clone(),
            target_port,
        },
        label: format!("{listen_addr}:{used_port} ← {target_host}:{target_port}"),
    };

    fwd_state
        .insert_entry(ForwardEntry {
            info: info.clone(),
            accept_abort: None,
            bridge_aborts: Vec::new(),
        })
        .await;

    log::info!("remote forward {forward_id} started: {listen_addr}:{used_port} ← {target_host}:{target_port}");
    Ok(info)
}

/// Stop a port forwarding tunnel.
#[command]
pub async fn port_forward_stop(
    app_handle: AppHandle,
    fwd_state: tauri::State<'_, ForwardingState>,
    forward_id: String,
) -> Result<(), String> {
    #[cfg(not(any(target_os = "ios", target_os = "android")))]
    {
        return stop_impl(app_handle, fwd_state, forward_id).await;
    }
    #[cfg(any(target_os = "ios", target_os = "android"))]
    {
        let _ = (app_handle, fwd_state, forward_id);
        Err("port forwarding not available on mobile".into())
    }
}

#[cfg(not(any(target_os = "ios", target_os = "android")))]
async fn stop_impl(
    app_handle: AppHandle,
    fwd_state: tauri::State<'_, ForwardingState>,
    forward_id: String,
) -> Result<(), String> {
    let info = fwd_state
        .get_entry(&forward_id)
        .await
        .map(|e| e.info)
        .ok_or_else(|| ForwardError::ForwardNotFound(forward_id.clone()).to_string())?;

    if let ForwardKind::Remote {
        ref remote_listen_addr,
        remote_listen_port,
        ..
    } = info.kind
    {
        let ssh_state = app_handle.state::<SshState>();
        let _ = ssh_state
            .cancel_tcpip_forward(
                &info.session_id,
                remote_listen_addr,
                u32::from(remote_listen_port),
            )
            .await;
    }

    fwd_state.remove_and_abort(&forward_id).await;
    log::info!("forward {forward_id} stopped");
    Ok(())
}

/// List active port forwarding tunnels.
#[command]
pub async fn port_forward_list(
    fwd_state: tauri::State<'_, ForwardingState>,
    session_id: Option<String>,
) -> Result<Vec<ForwardInfo>, String> {
    #[cfg(not(any(target_os = "ios", target_os = "android")))]
    {
        return Ok(fwd_state.list_infos(session_id.as_deref()).await);
    }
    #[cfg(any(target_os = "ios", target_os = "android"))]
    {
        let _ = (fwd_state, session_id);
        Ok(vec![])
    }
}

/// Start a dynamic (SOCKS5) port forwarding tunnel.
#[command]
#[allow(clippy::too_many_arguments)]
pub async fn port_forward_start_dynamic(
    app_handle: AppHandle,
    fwd_state: tauri::State<'_, ForwardingState>,
    session_id: String,
    local_addr: String,
    local_port: u16,
) -> Result<ForwardInfo, String> {
    #[cfg(not(any(target_os = "ios", target_os = "android")))]
    {
        return start_dynamic_impl(app_handle, fwd_state, session_id, local_addr, local_port).await;
    }
    #[cfg(any(target_os = "ios", target_os = "android"))]
    {
        let _ = (app_handle, fwd_state, session_id, local_addr, local_port);
        Err("port forwarding not available on mobile".into())
    }
}

#[cfg(not(any(target_os = "ios", target_os = "android")))]
async fn start_dynamic_impl(
    app_handle: AppHandle,
    fwd_state: tauri::State<'_, ForwardingState>,
    session_id: String,
    local_addr: String,
    local_port: u16,
) -> Result<ForwardInfo, String> {
    let addr = format!("{local_addr}:{local_port}");

    {
        let infos = fwd_state.list_infos(Some(&session_id)).await;
        if infos.iter().any(
            |i| matches!(&i.kind, ForwardKind::Dynamic { local_port: p, .. } if *p == local_port),
        ) {
            return Err(ForwardError::DuplicateForward(local_addr, local_port).into());
        }
    }

    let listener = tokio::net::TcpListener::bind(&addr)
        .await
        .map_err(|e| ForwardError::Io(e).to_string())?;

    let forward_id = db::new_table_row_id();
    let info = ForwardInfo {
        id: forward_id.clone(),
        session_id: session_id.clone(),
        kind: ForwardKind::Dynamic {
            local_addr: local_addr.clone(),
            local_port,
        },
        label: format!("{local_addr}:{local_port} (SOCKS5)"),
    };

    let app = app_handle.clone();
    let sid = session_id.clone();
    let accept_abort = tokio::spawn(async move {
        accept_loop_dynamic(app, listener, sid).await;
    })
    .abort_handle();

    fwd_state
        .insert_entry(ForwardEntry {
            info: info.clone(),
            accept_abort: Some(accept_abort),
            bridge_aborts: Vec::new(),
        })
        .await;

    log::info!("dynamic forward {forward_id} started: {local_addr}:{local_port}");
    Ok(info)
}

// ---------------------------------------------------------------------------
// Error type
// ---------------------------------------------------------------------------

#[derive(Debug, thiserror::Error)]
pub enum ForwardError {
    #[error("SSH error: {0}")]
    Ssh(#[from] russh::Error),
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("session not found: {0}")]
    SessionNotFound(String),
    #[error("forward not found: {0}")]
    ForwardNotFound(String),
    #[error("a forward already exists for {0}:{1}")]
    DuplicateForward(String, u16),
    #[error("SOCKS5 protocol error: {0}")]
    Socks5Protocol(String),
    #[error("server rejected TCP forwarding (AllowTcpForwarding may be disabled)")]
    ForwardingDisabled,
}

impl From<ForwardError> for String {
    fn from(e: ForwardError) -> Self {
        e.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_reply_format() {
        let reply = build_reply(0x00);
        assert_eq!(reply.len(), 10);
        assert_eq!(reply[0], 0x05); // SOCKS version
        assert_eq!(reply[1], 0x00); // reply code
        assert_eq!(reply[2], 0x00); // reserved
        assert_eq!(reply[3], 0x01); // IPv4 address type
                                    // BND.ADDR = 0.0.0.0
        assert_eq!(&reply[4..8], &[0, 0, 0, 0]);
        // BND.PORT = 0
        assert_eq!(&reply[8..10], &[0, 0]);
    }

    #[test]
    fn test_build_reply_with_reply_code() {
        let reply = build_reply(0x05); // CONNECTION_REFUSED
        assert_eq!(reply[1], 0x05);
    }

    #[test]
    fn test_socks5_reply_format() {
        use std::net::{Ipv4Addr, SocketAddr};
        let addr = SocketAddr::new(std::net::IpAddr::V4(Ipv4Addr::LOCALHOST), 1080);
        let reply = socks5_reply(socks5_reply_code::SUCCEEDED, addr);
        assert_eq!(reply.len(), 10);
        assert_eq!(reply[0], 0x05); // SOCKS version
        assert_eq!(reply[1], 0x00); // SUCCEEDED
        assert_eq!(reply[2], 0x00); // reserved
        assert_eq!(reply[3], 0x01); // IPv4
    }

    #[test]
    fn test_socks5_reply_with_error_code() {
        use std::net::{Ipv4Addr, SocketAddr};
        let addr = SocketAddr::new(std::net::IpAddr::V4(Ipv4Addr::new(0, 0, 0, 1)), 0);
        let reply = socks5_reply(socks5_reply_code::CONNECTION_REFUSED, addr);
        assert_eq!(reply[1], 0x05);
    }

    #[test]
    fn test_forward_error_conversion() {
        let err = ForwardError::SessionNotFound("test-session".to_string());
        let s: String = err.into();
        assert!(s.contains("session not found"));
        assert!(s.contains("test-session"));
    }

    #[test]
    fn test_forward_error_duplicate() {
        let err = ForwardError::DuplicateForward("127.0.0.1".to_string(), 8080);
        let s: String = err.into();
        assert!(s.contains("127.0.0.1"));
        assert!(s.contains("8080"));
    }

    #[test]
    fn test_forward_error_forwarding_disabled() {
        let err = ForwardError::ForwardingDisabled;
        let s: String = err.into();
        assert!(s.contains("AllowTcpForwarding"));
    }

    #[test]
    fn test_forward_error_socks5_protocol() {
        let err = ForwardError::Socks5Protocol("unsupported version: 4".to_string());
        let s: String = err.into();
        assert!(s.contains("unsupported version: 4"));
    }

    #[test]
    fn test_forward_error_ssh() {
        let err = ForwardError::Ssh(russh::Error::IO(std::io::Error::other("connection lost")));
        let s: String = err.into();
        assert!(s.contains("connection lost"));
    }

    #[test]
    fn test_forward_error_io() {
        let err = ForwardError::Io(std::io::Error::from_raw_os_error(2));
        let s: String = err.into();
        assert!(!s.is_empty());
    }

    #[test]
    fn test_forward_error_forward_not_found() {
        let err = ForwardError::ForwardNotFound("fw-123".to_string());
        let s: String = err.into();
        assert!(s.contains("fw-123"));
    }

    #[test]
    fn test_forward_info_serialization() {
        let info = ForwardInfo {
            id: "abc-123".to_string(),
            session_id: "sess-1".to_string(),
            kind: ForwardKind::Local {
                local_addr: "127.0.0.1".to_string(),
                local_port: 8080,
                remote_host: "db.internal".to_string(),
                remote_port: 5432,
            },
            label: "127.0.0.1:8080 → db.internal:5432".to_string(),
        };
        let json = serde_json::to_string(&info).unwrap();
        assert!(json.contains("abc-123"));
        assert!(json.contains("sess-1"));
        assert!(json.contains("127.0.0.1"));
    }

    #[test]
    fn test_forward_info_remote_serialization() {
        let info = ForwardInfo {
            id: "rem-1".to_string(),
            session_id: "sess-2".to_string(),
            kind: ForwardKind::Remote {
                remote_listen_addr: "0.0.0.0".to_string(),
                remote_listen_port: 9000,
                target_host: "localhost".to_string(),
                target_port: 3000,
            },
            label: "0.0.0.0:9000 ← localhost:3000".to_string(),
        };
        let json = serde_json::to_string(&info).unwrap();
        assert!(json.contains("rem-1"));
        assert!(json.contains("9000"));
    }

    #[test]
    fn test_forward_info_dynamic_serialization() {
        let info = ForwardInfo {
            id: "dyn-1".to_string(),
            session_id: "sess-3".to_string(),
            kind: ForwardKind::Dynamic {
                local_addr: "127.0.0.1".to_string(),
                local_port: 1080,
            },
            label: "127.0.0.1:1080 (SOCKS5)".to_string(),
        };
        let json = serde_json::to_string(&info).unwrap();
        assert!(json.contains("dyn-1"));
        assert!(json.contains("1080"));
    }
}
