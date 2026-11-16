import { useCallback, useEffect, useRef, useState } from "react";
import { errorMsg } from "../api/ipc";
import { useStore } from "../state/store";
import type { Msg } from "../i18n";

/** Steam data a page shows: loaded when the page opens and whenever `key` changes (another account, another
 *  language), and again on `reload`. A failed load keeps what was shown and tells why in `error`. */
export function useLoad<R>(load: () => Promise<R>, key: string) {
  const { t } = useStore();
  const [data, setData] = useState<R | null>(null);
  const [error, setError] = useState<Msg | null>(null);
  const [loading, setLoading] = useState(false);
  const loadRef = useRef(load);
  loadRef.current = load;
  // Only the latest load's answer counts: a slow earlier one must not overwrite it.
  const seq = useRef(0);

  const reload = useCallback(async () => {
    const mine = ++seq.current;
    setLoading(true);
    try {
      const result = await loadRef.current();
      if (mine === seq.current) {
        setData(result);
        setError(null);
      }
    } catch (err) {
      if (mine === seq.current) setError(errorMsg(err));
    } finally {
      if (mine === seq.current) setLoading(false);
    }
  }, []);

  useEffect(() => {
    setData(null);
    void reload();
  }, [key, reload]);

  return { data, loading, reload, error: error && t.msg(error), errorCode: error?.code ?? null };
}
