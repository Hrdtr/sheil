<script setup lang="ts">
import type { UnlistenFn } from '@tauri-apps/api/event';
import {
  ArrowUpIcon,
  FileIcon,
  FolderIcon,
  FolderPlusIcon,
  RefreshCwIcon,
  UploadIcon,
  MoreVerticalIcon,
  DownloadIcon,
  EditIcon,
  TrashIcon,
  HomeIcon,
} from '@lucide/vue';
import { getCurrentWebviewWindow } from '@tauri-apps/api/webviewWindow';

const { activeSession } = useSessions();
const {
  currentPath,
  entries,
  loading,
  error,
  panelOpen,
  open,
  close,
  refresh,
  navigate,
  goUp,
  createDirectory,
  deleteEntry,
  renameEntry,
} = useSftp();

type SftpEntry = (typeof entries.value)[number];

// --- Lifecycle: connect/disconnect SFTP when panel opens/closes or tab switches ---
let currentSftpSession: string | null = null;

watch(
  () => ({
    isOpen: panelOpen.value,
    sessionId: activeSession.value?.sshSessionId ?? null,
    isConnected: activeSession.value?.state === 'connected',
  }),
  async ({ isOpen, sessionId, isConnected }) => {
    const shouldConnect = isOpen && sessionId && isConnected;
    const sessionChanged = sessionId !== currentSftpSession;

    if (currentSftpSession && (!shouldConnect || sessionChanged)) {
      await close(currentSftpSession).catch(() => {});
      currentSftpSession = null;
    }

    if (shouldConnect && !currentSftpSession) {
      try {
        await open(sessionId!);
        currentSftpSession = sessionId!;
      } catch (e) {
        console.error('SFTP open failed:', e);
      }
    }
  },
  { immediate: true },
);

// --- Actions ---
async function handleRefresh() {
  const sessionId = activeSession.value?.sshSessionId;
  if (!sessionId) return;
  await refresh(sessionId);
}

async function handleNavigate(path: string) {
  const sessionId = activeSession.value?.sshSessionId;
  if (!sessionId) return;
  await navigate(sessionId, path);
}

async function handleGoUp() {
  const sessionId = activeSession.value?.sshSessionId;
  if (!sessionId) return;
  await goUp(sessionId);
}

async function handleGoHome() {
  const sessionId = activeSession.value?.sshSessionId;
  if (!sessionId) return;
  try {
    const home = await commands.sftp.canonicalize(sessionId, '.');
    await navigate(sessionId, home);
  } catch {
    await navigate(sessionId, '/');
  }
}

// --- New folder ---
const newFolderOpen = ref(false);
const newFolderName = ref('');

async function handleCreateFolder() {
  const name = newFolderName.value.trim();
  if (!name) return;
  const sessionId = activeSession.value?.sshSessionId;
  if (!sessionId) return;
  try {
    await createDirectory(sessionId, name);
    newFolderOpen.value = false;
    newFolderName.value = '';
  } catch (e) {
    toast.error(String(e));
  }
}

// --- Delete ---
const deleteConfirmOpen = ref(false);
const deleteTarget = ref<SftpEntry | null>(null);

function confirmDelete(entry: SftpEntry) {
  deleteTarget.value = entry;
  deleteConfirmOpen.value = true;
}

async function handleDelete() {
  const entry = deleteTarget.value;
  if (!entry) return;
  const sessionId = activeSession.value?.sshSessionId;
  if (!sessionId) return;
  try {
    await deleteEntry(sessionId, entry);
    deleteConfirmOpen.value = false;
    deleteTarget.value = null;
  } catch (e) {
    toast.error(String(e));
  }
}

// --- Rename ---
const renameOpen = ref(false);
const renameTarget = ref<SftpEntry | null>(null);
const renameValue = ref('');

function startRename(entry: SftpEntry) {
  renameTarget.value = entry;
  renameValue.value = entry.name;
  renameOpen.value = true;
}

async function handleRename() {
  const entry = renameTarget.value;
  const name = renameValue.value.trim();
  if (!entry || !name || name === entry.name) {
    renameOpen.value = false;
    return;
  }
  const sessionId = activeSession.value?.sshSessionId;
  if (!sessionId) return;
  try {
    await renameEntry(sessionId, entry.path, name);
    renameOpen.value = false;
    renameTarget.value = null;
  } catch (e) {
    toast.error(String(e));
  }
}

// --- Upload ---
async function handleUpload() {
  const sessionId = activeSession.value?.sshSessionId;
  if (!sessionId) return;
  try {
    const { open: openDialog } = await import('@tauri-apps/plugin-dialog');
    const selected = await openDialog({ multiple: false });
    if (!selected) return;
    const localPath = selected;
    const originalName =
      localPath.split('/').pop() || localPath.split('\\').pop() || 'uploaded_file';
    const fileName = uniqueName(originalName, new Set(entries.value.map((e) => e.name)));
    const remotePath = `${currentPath.value.replace(/\/$/, '')}/${fileName}`;
    await commands.sftp.upload(sessionId, localPath, remotePath);
    await refresh(sessionId);
    toast.success(`Uploaded ${fileName}`);
  } catch (e) {
    toast.error(String(e));
  }
}

// --- Download ---
async function handleDownload(entry: SftpEntry) {
  if (entry.isDir) return;
  const sessionId = activeSession.value?.sshSessionId;
  if (!sessionId) return;
  try {
    const { save } = await import('@tauri-apps/plugin-dialog');
    const selected = await save({ defaultPath: entry.name });
    if (!selected) return;
    await commands.sftp.download(sessionId, entry.path, selected);
    toast.success(`Downloaded ${entry.name}`);
  } catch (e) {
    toast.error(String(e));
  }
}

// --- Drag & drop upload ---
const isDragOver = ref(false);
let unlistenDrop: UnlistenFn | undefined;

onMounted(async () => {
  unlistenDrop = await getCurrentWebviewWindow().onDragDropEvent(async (event) => {
    const { type } = event.payload;

    // Only handle drops that land on the SFTP panel
    if (type !== 'leave') {
      const { position } = event.payload;
      const el = document.elementFromPoint(position.x, position.y);
      if (!el?.closest('[data-sftp-panel]')) {
        isDragOver.value = false;
        return;
      }
    }

    if (type === 'over') {
      isDragOver.value = true;
      return;
    }

    if (type === 'leave') {
      isDragOver.value = false;
      return;
    }

    if (type === 'drop' || type === 'enter') {
      isDragOver.value = false;
      if (type === 'enter') return;

      const paths = 'paths' in event.payload ? event.payload.paths : [];
      if (paths.length === 0) return;

      const sessionId = activeSession.value?.sshSessionId;
      if (!sessionId) return;

      const existingNames = new Set(entries.value.map((e) => e.name));

      for (const filePath of paths) {
        const originalName = filePath.split('/').pop() || filePath.split('\\').pop() || 'file';
        const fileName = uniqueName(originalName, existingNames);
        existingNames.add(fileName);
        const remotePath = `${currentPath.value.replace(/\/$/, '')}/${fileName}`;
        try {
          await commands.sftp.upload(sessionId, filePath, remotePath);
        } catch (err) {
          toast.error(`Failed to upload ${fileName}: ${String(err)}`);
        }
      }
      await refresh(sessionId);
      toast.success(`${paths.length} file(s) uploaded`);
    }
  });
});

onBeforeUnmount(() => {
  unlistenDrop?.();
});

function uniqueName(name: string, existing: Set<string>): string {
  if (!existing.has(name)) return name;
  const dot = name.indexOf('.');
  const base = dot === -1 ? name : name.slice(0, dot);
  const ext = dot === -1 ? '' : name.slice(dot);
  let i = 2;
  while (existing.has(`${base} (${i})${ext}`)) i++;
  return `${base} (${i})${ext}`;
}

// --- Helpers ---
function formatSize(bytes: number): string {
  if (bytes === 0) return '—';
  const units = ['B', 'KB', 'MB', 'GB', 'TB'];
  let i = 0;
  let size = bytes;
  while (size >= 1024 && i < units.length - 1) {
    size /= 1024;
    i++;
  }
  return `${size.toFixed(i === 0 ? 0 : 1)} ${units[i]}`;
}

function formatPermissions(mode: number): string {
  const typeChar =
    (mode & 0o170000) === 0o040000 ? 'd' : (mode & 0o170000) === 0o120000 ? 'l' : '-';
  const r = (m: number) => (mode & m ? 'r' : '-');
  const w = (m: number) => (mode & m ? 'w' : '-');
  const x = (m: number) => (mode & m ? 'x' : '-');
  return (
    typeChar +
    r(0o400) +
    w(0o200) +
    x(0o100) +
    r(0o040) +
    w(0o020) +
    x(0o010) +
    r(0o004) +
    w(0o002) +
    x(0o001)
  );
}

function formatDate(iso: string | null): string {
  if (!iso) return '—';
  try {
    const d = new Date(iso);
    return d.toLocaleDateString(undefined, {
      month: 'short',
      day: 'numeric',
      year: d.getFullYear() === new Date().getFullYear() ? undefined : 'numeric',
      hour: '2-digit',
      minute: '2-digit',
    });
  } catch {
    return iso;
  }
}

// --- Breadcrumbs ---
const breadcrumbs = computed(() => {
  const parts = currentPath.value.split('/').filter(Boolean);
  const crumbs: { name: string; path: string }[] = [{ name: '/', path: '/' }];
  let acc = '';
  for (const part of parts) {
    acc += '/' + part;
    crumbs.push({ name: part, path: acc });
  }
  return crumbs;
});
</script>

<template>
  <div
    v-if="panelOpen && activeSession?.sshSessionId && activeSession.state === 'connected'"
    class="flex flex-col shrink-0 rounded-lg relative"
    data-sftp-panel
  >
    <!-- Drag overlay -->
    <div
      v-if="isDragOver"
      class="absolute inset-0 z-10 flex items-center justify-center bg-primary/20 border-2 border-dashed border-primary rounded-lg"
    >
      <span class="text-sm font-medium text-primary">Drop files to upload</span>
    </div>
    <!-- Toolbar -->
    <div class="flex items-center gap-1 px-0 py-1">
      <Button variant="ghost" size="icon-sm" class="size-6" title="Home" @click="handleGoHome">
        <HomeIcon class="size-3.5" />
      </Button>
      <Button variant="ghost" size="icon-sm" class="size-6" title="Up" @click="handleGoUp">
        <ArrowUpIcon class="size-3.5" />
      </Button>
      <Separator orientation="vertical" class="h-2 ml-1 mr-2 my-auto" />
      <!-- Breadcrumbs -->
      <div class="flex items-center gap-0.5 text-xs overflow-x-auto flex-1 min-w-0">
        <template v-for="(crumb, i) in breadcrumbs" :key="crumb.path">
          <span v-if="i > 1" class="text-muted-foreground shrink-0">/</span>
          <button
            class="text-muted-foreground hover:text-foreground transition-colors shrink-0 px-0.5 rounded"
            :class="{ 'text-foreground font-medium': i === breadcrumbs.length - 1 }"
            @click="handleNavigate(crumb.path)"
          >
            {{ crumb.name }}
          </button>
        </template>
      </div>
      <Separator orientation="vertical" class="h-2 mr-1 ml-2 my-auto" />
      <Button variant="ghost" size="icon-sm" class="size-6" title="Refresh" @click="handleRefresh">
        <RefreshCwIcon :class="{ 'animate-spin': loading }" class="size-3.5" />
      </Button>
      <Button
        variant="ghost"
        size="icon-sm"
        class="size-6"
        title="New folder"
        @click="newFolderOpen = true"
      >
        <FolderPlusIcon class="size-3.5" />
      </Button>
      <Button variant="ghost" size="icon-sm" class="size-6" title="Upload" @click="handleUpload">
        <UploadIcon class="size-3.5" />
      </Button>
    </div>

    <!-- File list -->
    <div class="flex-1 overflow-y-auto max-h-[16vh] text-xs">
      <!-- Loading -->
      <div v-if="loading" class="flex items-center gap-2 px-1.25 py-1.5 text-muted-foreground">
        <RefreshCwIcon class="size-3 animate-spin" />
        Loading…
      </div>

      <!-- Error -->
      <div v-else-if="error" class="px-1.25 py-1.5 text-destructive text-xs">
        {{ error }}
        <Button variant="link" size="sm" class="text-xs h-auto px-1" @click="handleRefresh"
          >Retry</Button
        >
      </div>

      <!-- Empty -->
      <div
        v-else-if="entries.length === 0"
        class="px-1.25 py-1.5 text-center text-muted-foreground"
      >
        This directory is empty.
      </div>

      <!-- Entries -->
      <div v-else class="flex flex-col">
        <ContextMenu v-for="entry in entries" :key="entry.path">
          <ContextMenuTrigger as-child>
            <button
              class="flex items-center gap-2 pl-1.25 pr-2.5 py-1.5 hover:bg-accent/50 rounded-sm text-left group cursor-pointer transition-colors"
              @dblclick="entry.isDir ? handleNavigate(entry.path) : handleDownload(entry)"
            >
              <!-- Icon -->
              <FolderIcon v-if="entry.isDir" class="size-3.5 text-amber-500 shrink-0" />
              <FileIcon v-else class="size-3.5 text-muted-foreground shrink-0" />

              <!-- Name -->
              <span class="flex-1 truncate min-w-0" :title="entry.name">{{ entry.name }}</span>

              <!-- Size -->
              <span class="text-muted-foreground w-16 text-right shrink-0 tabular-nums">
                {{ entry.isDir ? '—' : formatSize(entry.size) }}
              </span>

              <!-- Modified -->
              <span
                class="text-muted-foreground w-32 text-right shrink-0 hidden sm:block tabular-nums truncate"
                :title="formatDate(entry.modified)"
              >
                {{ formatDate(entry.modified) }}
              </span>

              <!-- Permissions -->
              <span class="text-muted-foreground w-24 shrink-0 hidden md:block tabular-nums">
                {{ formatPermissions(entry.permissions) }}
              </span>

              <!-- Context menu button -->
              <DropdownMenu>
                <DropdownMenuTrigger as-child>
                  <Button
                    variant="ghost"
                    size="icon-sm"
                    class="size-5 opacity-0 group-hover:opacity-100 transition-opacity shrink-0"
                    @click.stop
                  >
                    <MoreVerticalIcon class="size-3" />
                  </Button>
                </DropdownMenuTrigger>
                <DropdownMenuContent align="end" class="w-40">
                  <template v-if="!entry.isDir">
                    <DropdownMenuItem @click="handleDownload(entry)">
                      <DownloadIcon class="size-3.5" />
                      Download
                    </DropdownMenuItem>
                  </template>
                  <DropdownMenuItem @click="startRename(entry)">
                    <EditIcon class="size-3.5" />
                    Rename
                  </DropdownMenuItem>
                  <DropdownMenuSeparator />
                  <DropdownMenuItem variant="destructive" @click="confirmDelete(entry)">
                    <TrashIcon class="size-3.5" />
                    Delete
                  </DropdownMenuItem>
                </DropdownMenuContent>
              </DropdownMenu>
            </button>
          </ContextMenuTrigger>
          <ContextMenuContent class="w-40">
            <template v-if="!entry.isDir">
              <ContextMenuItem @click="handleDownload(entry)">
                <DownloadIcon class="size-3.5" />
                Download
              </ContextMenuItem>
            </template>
            <ContextMenuItem @click="startRename(entry)">
              <EditIcon class="size-3.5" />
              Rename
            </ContextMenuItem>
            <ContextMenuSeparator />
            <ContextMenuItem variant="destructive" @click="confirmDelete(entry)">
              <TrashIcon class="size-3.5" />
              Delete
            </ContextMenuItem>
          </ContextMenuContent>
        </ContextMenu>
      </div>
    </div>

    <!-- New folder dialog -->
    <ResponsiveModal v-model:open="newFolderOpen" v-slot="{ kind }">
      <ResponsiveModalContent :class="kind === 'dialog' ? 'max-w-md' : ''">
        <ResponsiveModalHeader>
          <ResponsiveModalTitle>New Folder</ResponsiveModalTitle>
          <ResponsiveModalDescription
            >Create a new directory in {{ currentPath }}</ResponsiveModalDescription
          >
        </ResponsiveModalHeader>
        <div class="flex items-center gap-2">
          <Input
            v-model="newFolderName"
            placeholder="Folder name"
            @keyup.enter="handleCreateFolder"
          />
        </div>
        <ResponsiveModalFooter :class="kind === 'drawer' ? 'flex-col-reverse' : ''">
          <ResponsiveModalClose as-child>
            <Button variant="outline">Cancel</Button>
          </ResponsiveModalClose>
          <Button @click="handleCreateFolder">Create</Button>
        </ResponsiveModalFooter>
      </ResponsiveModalContent>
    </ResponsiveModal>

    <!-- Delete confirm dialog -->
    <AlertDialog v-model:open="deleteConfirmOpen">
      <AlertDialogContent>
        <AlertDialogHeader>
          <AlertDialogTitle>Delete {{ deleteTarget?.name }}?</AlertDialogTitle>
          <AlertDialogDescription>
            <template v-if="deleteTarget?.isDir">
              This will permanently delete the directory and its contents.
            </template>
            <template v-else>
              This will permanently delete the file. This action cannot be undone.
            </template>
          </AlertDialogDescription>
        </AlertDialogHeader>
        <AlertDialogFooter>
          <AlertDialogCancel variant="outline">Cancel</AlertDialogCancel>
          <AlertDialogAction variant="destructive" @click="handleDelete">Delete</AlertDialogAction>
        </AlertDialogFooter>
      </AlertDialogContent>
    </AlertDialog>

    <!-- Rename dialog -->
    <ResponsiveModal v-model:open="renameOpen" v-slot="{ kind }">
      <ResponsiveModalContent :class="kind === 'dialog' ? 'max-w-md' : ''">
        <ResponsiveModalHeader>
          <ResponsiveModalTitle>Rename</ResponsiveModalTitle>
          <ResponsiveModalDescription
            >Enter a new name for {{ renameTarget?.name }}</ResponsiveModalDescription
          >
        </ResponsiveModalHeader>
        <div class="flex items-center gap-2">
          <Input v-model="renameValue" placeholder="New name" @keyup.enter="handleRename" />
        </div>
        <ResponsiveModalFooter :class="kind === 'drawer' ? 'flex-col-reverse' : ''">
          <ResponsiveModalClose as-child>
            <Button variant="outline">Cancel</Button>
          </ResponsiveModalClose>
          <Button @click="handleRename">Rename</Button>
        </ResponsiveModalFooter>
      </ResponsiveModalContent>
    </ResponsiveModal>
  </div>
</template>
