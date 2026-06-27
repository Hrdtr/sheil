import { invoke } from '@tauri-apps/api/core';

export type SshAuth = { type: 'password'; value: string } | { type: 'key'; value: string };

export interface ImportedKeyInfo {
  name: string;
  key_type: string;
  fingerprint: string;
}

export async function sshConnect(
  host: string,
  port: number,
  username: string,
  auth: SshAuth,
): Promise<string> {
  return invoke<string>('ssh_connect', { host, port, username, auth });
}

export async function sshDisconnect(sessionId: string): Promise<void> {
  return invoke('ssh_disconnect', { sessionId });
}

export async function sshImportKey(name: string, keyData: string): Promise<ImportedKeyInfo> {
  return invoke<ImportedKeyInfo>('ssh_import_key', { name, keyData });
}

export async function sshListSessions(): Promise<string[]> {
  return invoke<string[]>('ssh_list_sessions');
}

export async function sshOpenChannel(sessionId: string, cols: number, rows: number): Promise<void> {
  return invoke('ssh_open_channel', { sessionId, cols, rows });
}

export async function sshWrite(sessionId: string, data: Uint8Array): Promise<void> {
  return invoke('ssh_write', { sessionId, data: Array.from(data) });
}

export async function sshResize(sessionId: string, cols: number, rows: number): Promise<void> {
  return invoke('ssh_resize', { sessionId, cols, rows });
}

export async function sshCloseChannel(sessionId: string): Promise<void> {
  return invoke('ssh_close_channel', { sessionId });
}
