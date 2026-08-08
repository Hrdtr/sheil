import { relaunch } from '@tauri-apps/plugin-process';
import { check, type Update } from '@tauri-apps/plugin-updater';
import { toast } from '@/utils/toast';

function _useUpdater() {
  const checking = ref(false);
  const installing = ref(false);
  const progress = ref(0);
  const pendingUpdate = useState<Update | null>('updater:pending', () => null);
  const lastChecked = useState<Date | null>('updater:last-checked', () => null);

  async function checkForUpdates(manual = false) {
    if (checking.value || installing.value) return;
    checking.value = true;
    try {
      const update = await check();
      lastChecked.value = new Date();
      if (update) {
        pendingUpdate.value = markRaw(update);
        toast.info(`Sheil v${update.version} is available`, {
          description: 'Download and install it now?',
          duration: 10_000,
          action: {
            label: 'Install',
            onClick: () => installUpdate(),
          },
        });
      } else if (manual) {
        toast.success('You are up to date', {
          description: 'No new version is available right now.',
        });
      }
    } catch (err) {
      console.error('Update check failed:', err);
      if (manual) {
        toast.error('Could not check for updates', {
          description: err instanceof Error ? err.message : String(err),
        });
      }
    } finally {
      checking.value = false;
    }
  }

  async function installUpdate() {
    const update = pendingUpdate.value;
    if (!update || installing.value) return;
    installing.value = true;
    progress.value = 0;

    const toastId = toast.loading('Downloading update…', { description: '0%' });
    try {
      let downloaded = 0;
      let contentLength = 0;

      await update.downloadAndInstall((event) => {
        switch (event.event) {
          case 'Started':
            contentLength = event.data.contentLength ?? 0;
            break;
          case 'Progress':
            downloaded += event.data.chunkLength;
            if (contentLength > 0) {
              const pct = Math.min(100, Math.round((downloaded / contentLength) * 100));
              if (pct !== progress.value) {
                progress.value = pct;
                toast.loading('Downloading update…', { id: toastId, description: `${pct}%` });
              }
            }
            break;
          case 'Finished':
            progress.value = 100;
            break;
        }
      });

      pendingUpdate.value = null;
      toast.success('Update installed', {
        id: toastId,
        description: 'Restart Sheil to start using the new version.',
        duration: Number.POSITIVE_INFINITY,
        action: {
          label: 'Restart',
          onClick: () => relaunch(),
        },
      });
    } catch (err) {
      console.error('Update install failed:', err);
      toast.error('Update failed', {
        id: toastId,
        description: err instanceof Error ? err.message : String(err),
      });
    } finally {
      installing.value = false;
    }
  }

  return {
    checking,
    installing,
    progress,
    pendingUpdate,
    lastChecked,
    checkForUpdates,
    installUpdate,
  };
}

export const useUpdater = createSharedComposable(_useUpdater);
