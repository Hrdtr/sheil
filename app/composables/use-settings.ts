import { useDebounceFn } from '@vueuse/core';

export function useSettings<T extends object>(namespace: string): Ref<T> {
  const debouncedPersist = useDebounceFn((value: T) => {
    settingsStore
      .persist(namespace, value as unknown as Record<string, unknown>)
      .catch((error: unknown) => {
        console.error('failed to persist settings:', error);
      });
  }, 300);

  return computed<T>({
    get: () => settingsStore.namespaceSettings<T>(namespace),
    set: (value) => {
      settingsStore.applyLocal(namespace, value as unknown as Record<string, unknown>);
      debouncedPersist(value);
    },
  });
}
