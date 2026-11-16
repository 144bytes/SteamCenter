import { useCallback, useState } from "react";
import { errorMsg } from "../api/ipc";
import { useStore } from "../state/store";

/** How a window saves: one request. The window closes only when it went
 *  through, and a refusal is said inside it, by the fields - a notice that
 *  fades while the window has closed on what was typed helps nobody. The
 *  snapshot is fetched again either way, before the window goes on. */
export function useSubmit() {
  const { refresh, t } = useStore();
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const run = useCallback(async (request: () => Promise<unknown>): Promise<boolean> => {
    setBusy(true);
    setError(null);
    try {
      await request();
      return true;
    } catch (err) {
      setError(t.msg(errorMsg(err)));
      return false;
    } finally {
      await refresh();
      setBusy(false);
    }
  }, [refresh, t]);
  return { busy, error, run };
}
