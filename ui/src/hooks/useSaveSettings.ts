import { ipc } from "../api/ipc";
import { useStore } from "../state/store";
import type { Settings } from "../api/types";

/** Settings being saved: a change made meanwhile starts from them, not from a snapshot that hasn't caught up. */
let saving: Settings | null = null;

/** Save a modified copy of the settings. */
export function useSaveSettings() {
  const { act, snapshot } = useStore();
  return (change: (s: Settings) => void) => {
    const next: Settings = structuredClone(saving ?? snapshot!.settings);
    change(next);
    saving = next;
    return act(() => ipc.saveSettings(next)).finally(() => {
      if (saving === next) saving = null;
    });
  };
}
