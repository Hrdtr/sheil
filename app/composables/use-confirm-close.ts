import type { UnlistenFn } from '@tauri-apps/api/event';
import { getCurrentWindow } from '@tauri-apps/api/window';

function _useConfirmClose() {
  const { sessions } = useSessions();

  const enabled = useLocalStorage('settings:confirm-close', false);
  const destroy = ref(false);
  const confirmCloseDialogOpen = ref(false);

  let unlisten: UnlistenFn | undefined;

  onMounted(async () => {
    unlisten = await getCurrentWindow().onCloseRequested((event) => {
      const activeSessions = sessions.value.filter(
        (session) => session.state === 'connected' || session.state === 'connecting',
      );

      if (!enabled.value || activeSessions.length === 0) {
        return;
      }

      event.preventDefault();
      confirmCloseDialogOpen.value = true;
    });
  });

  onUnmounted(() => {
    unlisten?.();
  });

  function confirmClose() {
    unlisten?.();
    if (destroy.value) {
      getCurrentWindow().destroy();
    } else {
      getCurrentWindow().close();
    }
  }

  function cancelClose() {
    confirmCloseDialogOpen.value = false;
    destroy.value = false;
  }

  function showConfirmCloseDialog(options: { destroy?: boolean } = {}) {
    destroy.value = !!options.destroy;
    confirmCloseDialogOpen.value = true;
  }

  return {
    confirmCloseEnabled: enabled,
    confirmCloseDialogOpen,
    confirmClose,
    cancelClose,
    showConfirmCloseDialog,
  };
}

export const useConfirmClose = createSharedComposable(_useConfirmClose);
