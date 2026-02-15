import { listen, type Event, type UnlistenFn } from '@tauri-apps/api/event';
import { useEffect } from 'react';

const getErrorMessage = (error: unknown): string => {
  if (error instanceof Error) {
    return error.message;
  }

  return String(error);
};

export function useTauriEvent<T>(eventName: string, handler: (event: Event<T>) => void): void {
  useEffect(
    function subscribeToTauriEvent() {
      let isMounted = true;
      let unlisten: UnlistenFn | null = null;

      listen<T>(eventName, handler)
        .then((unsubscribe) => {
          if (isMounted) {
            unlisten = unsubscribe;
            return;
          }

          unsubscribe();
        })
        .catch((error: unknown) => {
          console.error(`Failed to listen for '${eventName}'`, getErrorMessage(error));
        });

      return function cleanupTauriEventListener() {
        isMounted = false;
        if (unlisten) {
          unlisten();
        }
      };
    },
    [eventName, handler],
  );
}
