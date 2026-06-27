import type { ImportedKeyInfo, SshAuth } from '$lib/commands.svelte';
import { sshConnect, sshDisconnect, sshImportKey, sshListSessions } from '$lib/commands.svelte';
import { describe, it, expect, beforeEach } from 'vitest';
import { mockInvoke } from '../../setup';

beforeEach(() => {
  mockInvoke.mockClear();
});

describe('sshConnect', () => {
  it('invokes ssh_connect with password auth', async () => {
    const expectedSid = '42';
    mockInvoke.mockResolvedValueOnce(expectedSid);

    const sid = await sshConnect('example.com', 22, 'user', {
      type: 'password',
      value: 's3cret',
    });

    expect(mockInvoke).toHaveBeenCalledOnce();
    expect(mockInvoke).toHaveBeenCalledWith('ssh_connect', {
      host: 'example.com',
      port: 22,
      username: 'user',
      auth: { type: 'password', value: 's3cret' } satisfies SshAuth,
    });
    expect(sid).toBe(expectedSid);
  });

  it('invokes ssh_connect with key auth', async () => {
    const expectedSid = '7';
    mockInvoke.mockResolvedValueOnce(expectedSid);

    const sid = await sshConnect('host.local', 2222, 'admin', {
      type: 'key',
      value: 'my-ed25519-key',
    });

    expect(mockInvoke).toHaveBeenCalledOnce();
    expect(mockInvoke).toHaveBeenCalledWith('ssh_connect', {
      host: 'host.local',
      port: 2222,
      username: 'admin',
      auth: { type: 'key', value: 'my-ed25519-key' } satisfies SshAuth,
    });
    expect(sid).toBe(expectedSid);
  });

  it('returns string session ID', async () => {
    mockInvoke.mockResolvedValueOnce('session-1');

    const result = await sshConnect('127.0.0.1', 2222, 'root', {
      type: 'password',
      value: 'pw',
    });

    expect(typeof result).toBe('string');
  });

  it('propagates invoke errors', async () => {
    mockInvoke.mockRejectedValueOnce(new Error('Connection refused'));

    await expect(
      sshConnect('bad.host', 22, 'u', { type: 'password', value: 'pw' }),
    ).rejects.toThrow('Connection refused');
  });
});

describe('sshDisconnect', () => {
  it('invokes ssh_disconnect with session ID', async () => {
    mockInvoke.mockResolvedValueOnce(undefined);

    await sshDisconnect('session-42');

    expect(mockInvoke).toHaveBeenCalledOnce();
    expect(mockInvoke).toHaveBeenCalledWith('ssh_disconnect', {
      sessionId: 'session-42',
    });
  });

  it('returns void on success', async () => {
    mockInvoke.mockResolvedValueOnce(undefined);

    const result = await sshDisconnect('any-session');

    expect(result).toBeUndefined();
  });

  it('propagates invoke errors', async () => {
    mockInvoke.mockRejectedValueOnce(new Error('Session not found'));

    await expect(sshDisconnect('ghost-session')).rejects.toThrow('Session not found');
  });
});

describe('sshImportKey', () => {
  it('invokes ssh_import_key with name and keyData', async () => {
    const mockInfo: ImportedKeyInfo = {
      name: 'my-key',
      key_type: 'ssh-ed25519',
      fingerprint: 'SHA256:abc123',
    };
    mockInvoke.mockResolvedValueOnce(mockInfo);

    const info = await sshImportKey('my-key', '-----BEGIN OPENSSH PRIVATE KEY-----');

    expect(mockInvoke).toHaveBeenCalledOnce();
    expect(mockInvoke).toHaveBeenCalledWith('ssh_import_key', {
      name: 'my-key',
      keyData: '-----BEGIN OPENSSH PRIVATE KEY-----',
    });
    expect(info).toEqual(mockInfo);
  });

  it('returns ImportedKeyInfo shape', async () => {
    const mockInfo: ImportedKeyInfo = {
      name: 'rsa-key',
      key_type: 'ssh-rsa',
      fingerprint: 'SHA256:def456',
    };
    mockInvoke.mockResolvedValueOnce(mockInfo);

    const info = await sshImportKey('rsa-key', 'PEM-DATA');

    expect(info).toHaveProperty('name');
    expect(info).toHaveProperty('key_type');
    expect(info).toHaveProperty('fingerprint');
    expect(info.name).toBe('rsa-key');
    expect(info.key_type).toBe('ssh-rsa');
    expect(info.fingerprint).toBe('SHA256:def456');
  });

  it('propagates invoke errors', async () => {
    mockInvoke.mockRejectedValueOnce(new Error('Invalid key format'));

    await expect(sshImportKey('bad', 'garbage')).rejects.toThrow('Invalid key format');
  });
});

describe('sshListSessions', () => {
  it('invokes ssh_list_sessions with no arguments', async () => {
    mockInvoke.mockResolvedValueOnce(['session-1', 'session-2']);

    const sessions = await sshListSessions();

    expect(mockInvoke).toHaveBeenCalledOnce();
    expect(mockInvoke).toHaveBeenCalledWith('ssh_list_sessions');
    expect(sessions).toEqual(['session-1', 'session-2']);
  });

  it('returns string array', async () => {
    const expected = ['a', 'b', 'c'];
    mockInvoke.mockResolvedValueOnce(expected);

    const sessions = await sshListSessions();

    expect(sessions).toEqual(expected);
    expect(Array.isArray(sessions)).toBe(true);
  });

  it('returns empty array when no sessions', async () => {
    mockInvoke.mockResolvedValueOnce([]);

    const sessions = await sshListSessions();

    expect(sessions).toEqual([]);
  });

  it('propagates invoke errors', async () => {
    mockInvoke.mockRejectedValueOnce(new Error('Internal error'));

    await expect(sshListSessions()).rejects.toThrow('Internal error');
  });
});
