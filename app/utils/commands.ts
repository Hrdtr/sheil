import { invoke } from '@tauri-apps/api/core';

// ── Hosts ───────────────────────────────────────────────────────────────

type AuthMethod = 'password' | 'key';
type Protocol = 'ssh';

interface Host {
  id: string;
  name: string;
  host: string;
  port: number;
  username: string;
  protocol: Protocol;
  group: string | null;
  authMethod: AuthMethod;
  keyName: string | null;
  tags: string[];
  hasPassword: boolean;
  createdAt: string;
  updatedAt: string;
}

interface HostInput {
  name: string;
  host: string;
  port?: number;
  username: string;
  protocol?: Protocol;
  group?: string | null;
  authMethod?: AuthMethod;
  keyName?: string | null;
  /** Plaintext password — encrypted and stored in SQLite. */
  password?: string | null;
  tags?: string[];
}

interface HostUpdate {
  name?: string;
  host?: string;
  port?: number;
  username?: string;
  protocol?: Protocol;
  group?: string | null;
  authMethod?: AuthMethod;
  keyName?: string | null;
  password?: string | null;
  tags?: string[];
}

const host = {
  create: async (input: HostInput): Promise<Host> => {
    return invoke<Host>('host_create', { input });
  },

  list: async (): Promise<Host[]> => {
    return invoke<Host[]>('host_list');
  },

  resolve: async (id: string): Promise<Host> => {
    return invoke<Host>('host_resolve', { id });
  },

  update: async (id: string, update: HostUpdate): Promise<Host> => {
    return invoke<Host>('host_update', { id, update });
  },

  delete: async (id: string): Promise<void> => {
    return invoke('host_delete', { id });
  },

  /** Decrypts and returns a host's stored password for SSH connection. */
  resolvePassword: async (id: string): Promise<string> => {
    return invoke<string>('host_resolve_password', { id });
  },

  export: async (): Promise<string> => {
    return invoke<string>('host_export');
  },

  import: async (
    json: string,
  ): Promise<{ imported: number; skipped: number; failed: string[] }> => {
    return invoke('host_import', { json });
  },
};

// ── SSH ─────────────────────────────────────────────────────────────────

type SshAuth = { type: 'password'; value: string } | { type: 'key'; value: string };

interface ImportedKey {
  name: string;
  keyType: string;
  fingerprint: string;
}

const ssh = {
  connect: async (
    host: string,
    port: number,
    username: string,
    auth: SshAuth,
    keepaliveInterval?: number | null,
    connectTimeout?: number | null,
  ): Promise<string> => {
    return invoke<string>('ssh_connect', {
      host,
      port,
      username,
      auth,
      keepaliveInterval: keepaliveInterval ?? null,
      connectTimeout: connectTimeout ?? null,
    });
  },

  disconnect: async (sessionId: string): Promise<void> => {
    return invoke('ssh_disconnect', { sessionId });
  },

  importKey: async (name: string, keyData: string, passphrase?: string): Promise<ImportedKey> => {
    return invoke<ImportedKey>('ssh_import_key', { name, keyData, passphrase: passphrase || null });
  },

  listKeys: async (): Promise<ImportedKey[]> => {
    return invoke<ImportedKey[]>('ssh_list_keys');
  },

  deleteKey: async (name: string): Promise<void> => {
    return invoke('ssh_delete_key', { name });
  },

  listSessions: async (): Promise<string[]> => {
    return invoke<string[]>('ssh_list_sessions');
  },

  openChannel: async (sessionId: string, cols: number, rows: number): Promise<void> => {
    return invoke('ssh_open_channel', { sessionId, cols, rows });
  },

  write: async (sessionId: string, data: Uint8Array): Promise<void> => {
    return invoke('ssh_write', { sessionId, data: Array.from(data) });
  },

  resize: async (sessionId: string, cols: number, rows: number): Promise<void> => {
    return invoke('ssh_resize', { sessionId, cols, rows });
  },

  closeChannel: async (sessionId: string): Promise<void> => {
    return invoke('ssh_close_channel', { sessionId });
  },

  clearKnownHosts: async (): Promise<number> => {
    return invoke<number>('known_host_clear_all');
  },
};

// ── Port Forwarding ──────────────────────────────────────────────────────

type ForwardKind =
  | { type: 'local'; localAddr: string; localPort: number; remoteHost: string; remotePort: number }
  | {
      type: 'remote';
      remoteListenAddr: string;
      remoteListenPort: number;
      targetHost: string;
      targetPort: number;
    }
  | { type: 'dynamic'; localAddr: string; localPort: number };

interface ForwardInfo {
  id: string;
  sessionId: string;
  kind: ForwardKind;
  label: string;
}

const portForward = {
  startLocal: async (
    sessionId: string,
    localAddr: string,
    localPort: number,
    remoteHost: string,
    remotePort: number,
  ): Promise<ForwardInfo> => {
    return invoke<ForwardInfo>('port_forward_start_local', {
      sessionId,
      localAddr,
      localPort,
      remoteHost,
      remotePort,
    });
  },

  startRemote: async (
    sessionId: string,
    listenAddr: string,
    listenPort: number,
    targetHost: string,
    targetPort: number,
  ): Promise<ForwardInfo> => {
    return invoke<ForwardInfo>('port_forward_start_remote', {
      sessionId,
      listenAddr,
      listenPort,
      targetHost,
      targetPort,
    });
  },

  startDynamic: async (
    sessionId: string,
    localAddr: string,
    localPort: number,
  ): Promise<ForwardInfo> => {
    return invoke<ForwardInfo>('port_forward_start_dynamic', {
      sessionId,
      localAddr,
      localPort,
    });
  },

  stop: async (forwardId: string): Promise<void> => {
    return invoke('port_forward_stop', { forwardId });
  },

  list: async (sessionId?: string | null): Promise<ForwardInfo[]> => {
    return invoke<ForwardInfo[]>('port_forward_list', { sessionId: sessionId ?? null });
  },
};

// ── SFTP ──────────────────────────────────────────────────────────────────

interface SftpEntry {
  name: string;
  path: string;
  isDir: boolean;
  isSymlink: boolean;
  size: number;
  modified: string | null;
  permissions: number;
}

interface SftpMetadata {
  isDir: boolean;
  isSymlink: boolean;
  size: number;
  modified: string | null;
  accessed: string | null;
  permissions: number;
  owner: number | null;
  group: number | null;
}

const sftp = {
  connect: async (sessionId: string): Promise<void> => {
    return invoke('sftp_connect', { sessionId });
  },

  disconnect: async (sessionId: string): Promise<void> => {
    return invoke('sftp_disconnect', { sessionId });
  },

  listDir: async (sessionId: string, path: string): Promise<SftpEntry[]> => {
    return invoke<SftpEntry[]>('sftp_list_dir', { sessionId, path });
  },

  stat: async (sessionId: string, path: string): Promise<SftpMetadata> => {
    return invoke<SftpMetadata>('sftp_stat', { sessionId, path });
  },

  exists: async (sessionId: string, path: string): Promise<boolean> => {
    return invoke<boolean>('sftp_exists', { sessionId, path });
  },

  canonicalize: async (sessionId: string, path: string): Promise<string> => {
    return invoke<string>('sftp_canonicalize', { sessionId, path });
  },

  createDir: async (sessionId: string, path: string): Promise<void> => {
    return invoke('sftp_create_dir', { sessionId, path });
  },

  removeFile: async (sessionId: string, path: string): Promise<void> => {
    return invoke('sftp_remove_file', { sessionId, path });
  },

  removeDir: async (sessionId: string, path: string): Promise<void> => {
    return invoke('sftp_remove_dir', { sessionId, path });
  },

  rename: async (sessionId: string, oldPath: string, newPath: string): Promise<void> => {
    return invoke('sftp_rename', { sessionId, oldPath, newPath });
  },

  readFile: async (
    sessionId: string,
    path: string,
    offset: number,
    length: number,
  ): Promise<Uint8Array> => {
    const arr = await invoke<number[]>('sftp_read_file', { sessionId, path, offset, length });
    return new Uint8Array(arr);
  },

  writeFile: async (
    sessionId: string,
    path: string,
    data: Uint8Array,
    offset: number,
  ): Promise<void> => {
    return invoke('sftp_write_file', {
      sessionId,
      path,
      data: Array.from(data),
      offset,
    });
  },

  download: async (sessionId: string, remotePath: string, localPath: string): Promise<void> => {
    return invoke('sftp_download', { sessionId, remotePath, localPath });
  },

  upload: async (sessionId: string, localPath: string, remotePath: string): Promise<void> => {
    return invoke('sftp_upload', { sessionId, localPath, remotePath });
  },
};

// ── AI ──────────────────────────────────────────────────────────────────

interface AiModelFile {
  filename: string;
  sizeBytes: number;
}

const ai = {
  downloadModel: async (repo: string, filename: string): Promise<void> => {
    return invoke('ai_download_model', { repo, filename });
  },

  deleteModel: async (filename: string): Promise<void> => {
    return invoke('ai_delete_model', { filename });
  },

  listModels: async (): Promise<AiModelFile[]> => {
    return invoke<AiModelFile[]>('ai_list_models');
  },

  loadModel: async (filename: string, nCtx?: number): Promise<void> => {
    return invoke('ai_load_model', { filename, nCtx: nCtx ?? 2048 });
  },

  unloadModel: async (): Promise<void> => {
    return invoke('ai_unload_model');
  },

  generate: async (
    prompt: string,
    maxTokens: number,
    temperature: number,
    topP: number,
    mode: 'command' | 'complete',
  ): Promise<string> => {
    return invoke<string>('ai_generate', { prompt, maxTokens, temperature, topP, mode });
  },

  isLoaded: async (): Promise<boolean> => {
    return invoke<boolean>('ai_is_loaded');
  },
};

export const commands = {
  host,
  ssh,
  portForward,
  sftp,
  ai,
};
