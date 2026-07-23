type ForwardInfo = Awaited<ReturnType<typeof commands.portForward.startLocal>>;

function _usePortForwarding() {
  const forwards = ref<ForwardInfo[]>([]);
  const panelOpen = useState('portForwarding:panel-open', () => false);

  const togglePanel = () => {
    panelOpen.value = !panelOpen.value;
  };

  const startLocal = async (
    sessionId: string,
    localAddr: string,
    localPort: number,
    remoteHost: string,
    remotePort: number,
  ): Promise<ForwardInfo> => {
    const info = await commands.portForward.startLocal(
      sessionId,
      localAddr,
      localPort,
      remoteHost,
      remotePort,
    );
    forwards.value.push(info);
    return info;
  };

  const startRemote = async (
    sessionId: string,
    listenAddr: string,
    listenPort: number,
    targetHost: string,
    targetPort: number,
  ): Promise<ForwardInfo> => {
    const info = await commands.portForward.startRemote(
      sessionId,
      listenAddr,
      listenPort,
      targetHost,
      targetPort,
    );
    forwards.value.push(info);
    return info;
  };

  const startDynamic = async (
    sessionId: string,
    localAddr: string,
    localPort: number,
  ): Promise<ForwardInfo> => {
    const info = await commands.portForward.startDynamic(sessionId, localAddr, localPort);
    forwards.value.push(info);
    return info;
  };

  const stop = async (forwardId: string): Promise<void> => {
    await commands.portForward.stop(forwardId);
    forwards.value = forwards.value.filter((f) => f.id !== forwardId);
  };

  const refresh = async (sessionId?: string | null): Promise<void> => {
    forwards.value = await commands.portForward.list(sessionId);
  };

  return {
    forwards,
    panelOpen,
    togglePanel,
    startLocal,
    startRemote,
    startDynamic,
    stop,
    refresh,
  };
}

export const usePortForwarding = createSharedComposable(_usePortForwarding);
