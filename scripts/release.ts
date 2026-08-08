/// <reference types="node" />
import { spawnSync } from 'node:child_process';
import { existsSync, readFileSync, writeFileSync } from 'node:fs';
import { join, relative } from 'node:path';
import process from 'node:process';
import * as p from '@clack/prompts';

const ROOT = join(import.meta.dirname, '..');
const TAURI_DIR = join(ROOT, 'tauri');
const PACKAGE_JSON = join(ROOT, 'package.json');
const TAURI_CONF = join(TAURI_DIR, 'tauri.conf.json');
const CARGO_TOML = join(TAURI_DIR, 'Cargo.toml');
const CARGO_LOCK = join(TAURI_DIR, 'Cargo.lock');
const CHANGES_MD = join(ROOT, 'CHANGES.md');

// Version sync scope (desktop): package.json, tauri/tauri.conf.json, tauri/Cargo.toml,
// and tauri/Cargo.lock (via `cargo metadata`). tauri.conf.json is the source of truth
// that tauri-action uses for the release tag (__VERSION__).
//
// NOTE (mobile): iOS/Android sources are not generated yet (`tauri/gen` has no
// `apple`/`android` projects). Once `tauri ios init` / `tauri android init` have been
// run and mobile releases are enabled, extend this script to also sync:
//   - Android: tauri/gen/android/app/build.gradle -> versionCode (bump the integer)
//     and versionName (derived from tauri.conf.json, verify it matches).
//   - iOS: tauri/gen/apple project settings -> CFBundleShortVersionString /
//     CFBundleVersion (derived from tauri.conf.json, verify they match).
// Mobile builds read the app version from tauri.conf.json, so keeping that file bumped
// covers the display version; only the platform build numbers need extra handling.

const USAGE = [
  'Usage: pnpm release [patch | minor | major | <version>] [options]',
  '',
  'Options:',
  '  --dry-run   Show the release plan without making any changes',
  '  --yes, -y   Skip the confirmation prompt',
  '  --help, -h  Show this help',
].join('\n');

const SECTION_LABELS: Record<string, string> = {
  feat: 'Features',
  fix: 'Bug Fixes',
  perf: 'Performance Improvements',
  refactor: 'Refactoring',
  docs: 'Documentation',
};

interface SemVer {
  major: number;
  minor: number;
  patch: number;
  prerelease: string;
}

interface CommitEntry {
  type: string;
  scope: string;
  breaking: boolean;
  description: string;
  hash: string;
}

type BumpKind = 'major' | 'minor' | 'patch';

function run(
  cmd: string,
  args: string[],
  opts: { cwd?: string; ignoreOutput?: boolean } = {},
): string {
  const result = spawnSync(cmd, args, {
    cwd: opts.cwd ?? ROOT,
    encoding: 'utf8',
    stdio: opts.ignoreOutput ? 'ignore' : ['ignore', 'pipe', 'pipe'],
  });
  if (result.status !== 0) {
    const stderr = typeof result.stderr === 'string' ? result.stderr.trim() : '';
    throw new Error(`Command failed: ${cmd} ${args.join(' ')}\n${stderr}`);
  }
  return typeof result.stdout === 'string' ? result.stdout.trim() : '';
}

function parseVersion(raw: string): SemVer {
  const match = /^(\d+)\.(\d+)\.(\d+)(?:-([0-9A-Za-z-.]+))?$/.exec(raw.trim().replace(/^v/, ''));
  if (!match) throw new Error(`Invalid version: "${raw}"`);
  return {
    major: Number(match[1]),
    minor: Number(match[2]),
    patch: Number(match[3]),
    prerelease: match[4] ?? '',
  };
}

function formatVersion(version: SemVer): string {
  const base = `${version.major}.${version.minor}.${version.patch}`;
  return version.prerelease ? `${base}-${version.prerelease}` : base;
}

function compareVersions(a: SemVer, b: SemVer): number {
  for (const key of ['major', 'minor', 'patch'] as const) {
    if (a[key] !== b[key]) return a[key] - b[key];
  }
  if (a.prerelease === b.prerelease) return 0;
  if (!a.prerelease) return 1;
  if (!b.prerelease) return -1;
  return a.prerelease < b.prerelease ? -1 : 1;
}

function bumpVersion(current: SemVer, kind: BumpKind): SemVer {
  if (kind === 'major') return { major: current.major + 1, minor: 0, patch: 0, prerelease: '' };
  if (kind === 'minor')
    return { major: current.major, minor: current.minor + 1, patch: 0, prerelease: '' };
  if (current.prerelease) return { ...current, prerelease: '' };
  return { ...current, patch: current.patch + 1, prerelease: '' };
}

function preflight(dryRun: boolean): void {
  run('git', ['rev-parse', '--is-inside-work-tree']);
  const branch = run('git', ['branch', '--show-current']);
  if (branch !== 'main')
    throw new Error(`Release must be cut from "main" (currently on "${branch}").`);
  if (dryRun) {
    p.log.warn('Dry run: skipping clean tree and remote sync checks.');
    return;
  }
  const status = run('git', ['status', '--porcelain']);
  if (status) throw new Error('Working tree has uncommitted changes. Commit or stash them first.');
  try {
    run('git', ['fetch', 'origin', 'main']);
  } catch {
    throw new Error('Could not fetch origin/main. Check your network and remote configuration.');
  }
  const counts = run('git', ['rev-list', '--left-right', '--count', 'HEAD...origin/main']);
  const parts = counts.split(/\s+/);
  const ahead = Number(parts[0]);
  const behind = Number(parts[1]);
  if (behind > 0)
    throw new Error(`Local main is ${behind} commit(s) behind origin/main. Pull first.`);
  if (ahead > 0)
    throw new Error(`Local main is ${ahead} commit(s) ahead of origin/main. Push them first.`);
}

function readCurrentVersion(): string {
  const config = JSON.parse(readFileSync(TAURI_CONF, 'utf8')) as { version?: string };
  if (!config.version) throw new Error(`No version found in ${TAURI_CONF}`);
  return config.version;
}

function getLastTag(): string | undefined {
  try {
    return run('git', ['describe', '--tags', '--abbrev=0']) || undefined;
  } catch {
    return undefined;
  }
}

function collectCommits(lastTag: string | undefined): CommitEntry[] {
  const range = lastTag ? `${lastTag}..HEAD` : 'HEAD';
  let output = '';
  try {
    output = run('git', ['log', range, '--no-merges', '--pretty=format:%h%x00%s']);
  } catch {
    return [];
  }
  const entries: CommitEntry[] = [];
  for (const line of output.split('\n')) {
    if (!line) continue;
    const sep = line.indexOf('\0');
    if (sep === -1) continue;
    const hash = line.slice(0, sep);
    const subject = line.slice(sep + 1);
    const match = /^(\w+)(?:\(([^)]+)\))?(!)?:\s*(.+)$/.exec(subject);
    if (!match) continue;
    entries.push({
      type: match[1].toLowerCase(),
      scope: match[2] ?? '',
      breaking: match[3] === '!',
      description: match[4],
      hash,
    });
  }
  return entries;
}

function formatEntry(entry: CommitEntry): string {
  const scope = entry.scope ? `**${entry.scope}:** ` : '';
  return `- ${scope}${entry.description} (${entry.hash})`;
}

function renderChangelogSection(tag: string, entries: CommitEntry[]): string {
  const date = new Date().toISOString().slice(0, 10);
  const lines: string[] = [`## ${tag} (${date})`];
  const breaking = entries.filter((entry) => entry.breaking);
  if (breaking.length > 0) {
    lines.push('', '### Breaking Changes', ...breaking.map(formatEntry));
  }
  for (const [type, label] of Object.entries(SECTION_LABELS)) {
    const group = entries.filter((entry) => entry.type === type);
    if (group.length > 0) lines.push('', `### ${label}`, ...group.map(formatEntry));
  }
  return lines.length > 1 ? lines.join('\n') : '';
}

function writeChangelog(section: string): void {
  const existing = existsSync(CHANGES_MD) ? readFileSync(CHANGES_MD, 'utf8') : '';
  let content: string;
  if (!existing) {
    content = `# Changelog\n\n${section}\n`;
  } else {
    const lines = existing.split('\n');
    const headingIndex = lines.findIndex((line) => line.startsWith('# '));
    if (headingIndex === -1) {
      content = `${section}\n\n${existing}`;
    } else {
      const insert = lines[headingIndex + 1] === '' ? [section] : ['', section];
      lines.splice(headingIndex + 1, 0, ...insert);
      content = lines.join('\n');
    }
  }
  writeFileSync(CHANGES_MD, content);
}

function updateJsonVersion(filePath: string, version: string): void {
  const data = JSON.parse(readFileSync(filePath, 'utf8')) as Record<string, unknown>;
  data.version = version;
  writeFileSync(filePath, `${JSON.stringify(data, null, 2)}\n`);
}

function updateCargoVersion(filePath: string, version: string): void {
  const lines = readFileSync(filePath, 'utf8').split('\n');
  let inPackage = false;
  for (let i = 0; i < lines.length; i++) {
    const trimmed = lines[i].trim();
    if (trimmed.startsWith('[') && trimmed.endsWith(']')) {
      inPackage = trimmed === '[package]';
      continue;
    }
    if (inPackage && /^version\s*=/.test(trimmed)) {
      lines[i] = `version = "${version}"`;
      writeFileSync(filePath, lines.join('\n'));
      return;
    }
  }
  throw new Error(`No version field found under [package] in ${filePath}`);
}

function repoSlug(): string {
  try {
    const url = run('git', ['remote', 'get-url', 'origin']);
    const match = /github\.com[:/]([^/]+\/[^/.]+)/.exec(url);
    if (match) return match[1];
  } catch {}
  return 'Hrdtr/sheil';
}

function cancelled(): void {
  p.cancel('Release cancelled.');
}

async function resolveNextVersion(
  current: SemVer,
  positional: string | undefined,
): Promise<SemVer> {
  if (positional) {
    if (positional === 'patch' || positional === 'minor' || positional === 'major') {
      return bumpVersion(current, positional);
    }
    const explicit = parseVersion(positional);
    if (compareVersions(explicit, current) <= 0) {
      throw new Error(
        `Version ${formatVersion(explicit)} must be greater than ${formatVersion(current)}.`,
      );
    }
    return explicit;
  }
  const choice = await p.select<BumpKind | 'custom'>({
    message: `Current version is v${formatVersion(current)}. Which bump?`,
    options: [
      { value: 'patch', label: 'patch', hint: `v${formatVersion(bumpVersion(current, 'patch'))}` },
      { value: 'minor', label: 'minor', hint: `v${formatVersion(bumpVersion(current, 'minor'))}` },
      { value: 'major', label: 'major', hint: `v${formatVersion(bumpVersion(current, 'major'))}` },
      { value: 'custom', label: 'custom', hint: 'enter an explicit version' },
    ],
  });
  if (p.isCancel(choice)) {
    cancelled();
    process.exitCode = 0;
    throw new CancelledError();
  }
  if (choice !== 'custom') return bumpVersion(current, choice);
  const raw = await p.text({
    message: 'New version',
    initialValue: formatVersion(current),
    placeholder: 'x.y.z',
    validate: (value) => {
      try {
        const parsed = parseVersion(value || '');
        if (compareVersions(parsed, current) <= 0) {
          return `Must be greater than v${formatVersion(current)}`;
        }
        return undefined;
      } catch {
        return 'Invalid semver version';
      }
    },
  });
  if (p.isCancel(raw)) {
    cancelled();
    throw new CancelledError();
  }
  return parseVersion(raw);
}

class CancelledError extends Error {
  constructor() {
    super('cancelled');
    this.name = 'CancelledError';
  }
}

async function main(): Promise<void> {
  const argv = process.argv.slice(2);
  if (argv.includes('--help') || argv.includes('-h')) {
    p.note(USAGE, 'Sheil release');
    return;
  }
  const dryRun = argv.includes('--dry-run');
  const assumeYes = argv.includes('--yes') || argv.includes('-y');
  const positional = argv.find((arg) => !arg.startsWith('-'));

  p.intro('Sheil release');
  preflight(dryRun);

  const currentRaw = readCurrentVersion();
  const current = parseVersion(currentRaw);
  const next = await resolveNextVersion(current, positional);
  const nextRaw = formatVersion(next);
  const tag = `v${nextRaw}`;

  if (run('git', ['tag', '--list', tag])) {
    throw new Error(`Tag ${tag} already exists.`);
  }

  const lastTag = getLastTag();
  const entries = collectCommits(lastTag);
  const section = renderChangelogSection(tag, entries);

  const plan = [
    `Version:   v${currentRaw} → v${nextRaw}`,
    `Tag:       ${tag}`,
    `Commits:   ${entries.length}${lastTag ? ` since ${lastTag}` : ' (no previous tag)'}`,
    `Files:     package.json, tauri/tauri.conf.json,`,
    `           tauri/Cargo.toml, tauri/Cargo.lock${section ? ', CHANGES.md' : ''}`,
    '',
    ...(section ? section.split('\n').slice(0, 15) : ['(no changelog entries found)']),
  ].join('\n');
  p.note(plan, 'Release plan');

  if (dryRun) {
    p.outro('Dry run complete — no changes were made.');
    return;
  }

  if (!assumeYes) {
    const ok = await p.confirm({ message: `Release ${tag}?`, initialValue: true });
    if (p.isCancel(ok) || !ok) {
      cancelled();
      return;
    }
  }

  const s = p.spinner();
  try {
    s.start(`Bumping version to ${tag}`);
    updateJsonVersion(PACKAGE_JSON, nextRaw);
    updateJsonVersion(TAURI_CONF, nextRaw);
    updateCargoVersion(CARGO_TOML, nextRaw);
    run('cargo', ['metadata', '--format-version', '1'], { cwd: TAURI_DIR, ignoreOutput: true });
    s.stop(`Bumped version to ${tag}`);

    if (section) {
      s.start('Updating CHANGES.md');
      writeChangelog(section);
      run('pnpm', ['fmt', 'CHANGES.md']);
      s.stop('Updated CHANGES.md');
    } else {
      p.log.warn('No conventional commits found since the last tag; skipping CHANGES.md.');
    }

    s.start('Committing, tagging and pushing');
    const files = [PACKAGE_JSON, TAURI_CONF, CARGO_TOML, CARGO_LOCK].map((file) =>
      relative(ROOT, file),
    );
    if (section) files.push(relative(ROOT, CHANGES_MD));
    run('git', ['add', ...files]);
    run('git', ['commit', '-s', '-m', `chore(release): ${tag}`]);
    run('git', ['tag', '-a', tag, '-m', tag]);
    run('git', ['push', 'origin', 'main']);
    run('git', ['push', 'origin', tag]);
    s.stop(`Released ${tag}`);
  } catch (err) {
    s.error('Release failed');
    throw err;
  }

  const slug = repoSlug();
  p.note(
    `The tag push triggers the release workflow.\nWatch progress:\nhttps://github.com/${slug}/actions/workflows/release.yml`,
    'Done',
  );
  p.outro(`${tag} is on its way.`);
}

try {
  await main();
} catch (err) {
  if (!(err instanceof CancelledError)) {
    p.cancel(err instanceof Error ? err.message : String(err));
    process.exitCode = 1;
  }
}
