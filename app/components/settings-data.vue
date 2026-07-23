<script setup lang="ts">
import { save } from '@tauri-apps/plugin-dialog';
import { writeTextFile } from '@tauri-apps/plugin-fs';

const { clear: clearRecent } = useRecentHosts();
const { refreshHosts } = useHosts();

const clearingKnownHosts = ref(false);
async function handleClearKnownHosts() {
  clearingKnownHosts.value = true;
  try {
    const count = await commands.ssh.clearKnownHosts();
    toast.success(`Cleared ${count} saved host fingerprints.`);
  } catch (e) {
    toast.error(`Failed to clear known hosts: ${e}`);
  } finally {
    clearingKnownHosts.value = false;
  }
}

async function handleExport() {
  try {
    const path = await save({
      filters: [{ name: 'JSON', extensions: ['json'] }],
      defaultPath: 'sheil-hosts.json',
    });
    if (!path) return;
    const json = await commands.host.export();
    await writeTextFile(path, json);
    toast.success('Hosts exported.');
  } catch (e) {
    toast.error(`Export failed: ${e}`);
  }
}

const importing = ref(false);
const fileInput = useTemplateRef('fileInput');

function triggerImport() {
  fileInput.value?.click();
}

async function handleImport(event: Event) {
  const input = event.target as HTMLInputElement;
  const file = input.files?.[0];
  if (!file) return;

  importing.value = true;
  try {
    const text = await file.text();
    const result = await commands.host.import(text);
    toast.success(
      `Imported ${result.imported}, skipped ${result.skipped}, failed ${result.failed.length}.`,
    );
    if (result.failed.length > 0) {
      toast.error(`Failures: ${result.failed.join(', ')}`);
    }
    refreshHosts();
  } catch (e) {
    toast.error(`Import failed: ${e}`);
  } finally {
    importing.value = false;
    input.value = '';
  }
}
</script>

<template>
  <div class="flex flex-col gap-5">
    <div class="flex items-start justify-between gap-4">
      <Field>
        <FieldLabel>Clear Recent Hosts</FieldLabel>
        <FieldDescription
          >Clears the recent connections list on the welcome screen.</FieldDescription
        >
      </Field>
      <AlertDialog>
        <AlertDialogTrigger as-child>
          <Button variant="secondary" size="sm">Clear</Button>
        </AlertDialogTrigger>
        <AlertDialogContent>
          <AlertDialogHeader>
            <AlertDialogTitle>Clear Recent Hosts</AlertDialogTitle>
            <AlertDialogDescription>
              This will remove all recently connected hosts from the welcome screen.
            </AlertDialogDescription>
          </AlertDialogHeader>
          <AlertDialogFooter>
            <AlertDialogCancel variant="ghost">Cancel</AlertDialogCancel>
            <AlertDialogAction @click="clearRecent">Clear</AlertDialogAction>
          </AlertDialogFooter>
        </AlertDialogContent>
      </AlertDialog>
    </div>

    <Separator />

    <div class="flex items-start justify-between gap-4">
      <Field>
        <FieldLabel>Clear Known Hosts</FieldLabel>
        <FieldDescription
          >Remove all trusted server fingerprints. You will be prompted to verify hosts again on
          next connection.</FieldDescription
        >
      </Field>
      <AlertDialog>
        <AlertDialogTrigger as-child>
          <Button variant="secondary" size="sm">Clear</Button>
        </AlertDialogTrigger>
        <AlertDialogContent>
          <AlertDialogHeader>
            <AlertDialogTitle>Clear Known Hosts</AlertDialogTitle>
            <AlertDialogDescription>
              All stored server fingerprints will be deleted. You will need to verify host keys
              again on next connection.
            </AlertDialogDescription>
          </AlertDialogHeader>
          <AlertDialogFooter>
            <AlertDialogCancel variant="ghost">Cancel</AlertDialogCancel>
            <AlertDialogAction :disabled="clearingKnownHosts" @click="handleClearKnownHosts"
              >Clear</AlertDialogAction
            >
          </AlertDialogFooter>
        </AlertDialogContent>
      </AlertDialog>
    </div>

    <Separator />

    <div class="flex items-start justify-between gap-4">
      <Field>
        <FieldLabel>Export Hosts</FieldLabel>
        <FieldDescription
          >Download host configurations as a JSON file. Passwords are excluded.</FieldDescription
        >
      </Field>
      <Button variant="secondary" size="sm" @click="handleExport">Export</Button>
    </div>

    <Separator />

    <div class="flex items-start justify-between gap-4">
      <Field>
        <FieldLabel>Import Hosts</FieldLabel>
        <FieldDescription
          >Import host configurations from a Sheil JSON export file.</FieldDescription
        >
      </Field>
      <div>
        <input ref="fileInput" type="file" accept=".json" class="hidden" @change="handleImport" />
        <Button variant="secondary" size="sm" :disabled="importing" @click="triggerImport">
          {{ importing ? 'Importing…' : 'Import' }}
        </Button>
      </div>
    </div>
  </div>
</template>
