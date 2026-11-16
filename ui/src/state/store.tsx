/** Application state.
 *
 *  The single source of truth is the snapshot computed by the Rust side.
 *  Events don't modify the store, they only say "data changed" - and we re-read the snapshot.
 *  The log lives in its own context: a new line re-renders only the page showing it.
 *  Steam data (library, profile, wishlist) is loaded by the page that shows it (`useLoad`).
 */
import {
  createContext, useCallback, useContext, useEffect, useMemo, useRef, useState,
} from "react";
import type { ReactNode } from "react";
import { errorMsg, ipc, onLog, onNotice, onProgress, onState } from "../api/ipc";
import { makeT } from "../i18n";
import type { Lang, T } from "../i18n";
import type { LogLine, Progress, Snapshot } from "../api/types";

const MAX_LOGS = 1500;

interface Toast {
  id: number;
  kind: "ok" | "err";
  text: string;
}

interface Store {
  snapshot: Snapshot | null;
  t: T;
  toasts: Toast[];
  /** The running operation's progress (null - none, or it hasn't reported yet). */
  progress: Progress | null;
  refresh: () => Promise<void>;
  notify: (text: string, kind?: "ok" | "err") => void;
  /** Run an action; show an error as a toast, re-read the snapshot on success. */
  act: <R>(fn: () => Promise<R>, okText?: string) => Promise<R | null>;
  /** "Check for updates": null - idle; "" - checking; a version - downloading it (the app restarts when done). */
  update: string | null;
  /** Check and install a newer version. `quiet` - the check made by itself at startup: says nothing when there is
   *  none or the check fails, and tells why the app is about to restart when there is one. */
  checkUpdate: (quiet?: boolean) => Promise<void>;
}

/** A log line with a number of its own: the key that stays put while old lines drop off the front. */
type Line = LogLine & { id: number };

interface Logs {
  logs: Line[];
  clearLogs: () => void;
}

const Ctx = createContext<Store | null>(null);
const LogsCtx = createContext<Logs | null>(null);

export function useStore(): Store {
  const store = useContext(Ctx);
  if (!store) throw new Error("useStore outside provider");
  return store;
}

export function useLogs(): Logs {
  const logs = useContext(LogsCtx);
  if (!logs) throw new Error("useLogs outside provider");
  return logs;
}

/** `withLogs` - read the log (not in the tray menu's window, which never shows it). */
export function StoreProvider({ withLogs = true, children }: { withLogs?: boolean; children: ReactNode }) {
  const [snapshot, setSnapshot] = useState<Snapshot | null>(null);
  const [logs, setLogs] = useState<Line[]>([]);
  const [toasts, setToasts] = useState<Toast[]>([]);
  const [progress, setProgress] = useState<Progress | null>(null);
  const inflight = useRef<Promise<void> | null>(null);
  const next = useRef<Promise<void> | null>(null);
  const toastId = useRef(0);
  const lineId = useRef(0);

  /** Re-read the snapshot. Requests made during a read share one more read after it. A failed read keeps the old
   *  snapshot and never rejects, so the queue can't get stuck. */
  const refresh = useCallback((): Promise<void> => {
    if (inflight.current) {
      next.current ??= inflight.current.then(() => {
        next.current = null;
        return refresh();
      });
      return next.current;
    }
    inflight.current = ipc.state().then(setSnapshot, () => undefined).finally(() => {
      inflight.current = null;
    });
    return inflight.current;
  }, []);

  const lang: Lang = snapshot?.settings.language ?? "en";
  const t = useMemo(() => makeT(lang), [lang]);
  useEffect(() => { document.documentElement.lang = lang; }, [lang]);

  const notify = useCallback((text: string, kind: "ok" | "err" = "ok") => {
    const id = ++toastId.current;
    setToasts((list) => [...list, { id, kind, text }]);
    setTimeout(() => setToasts((list) => list.filter((x) => x.id !== id)),
      kind === "err" ? 6000 : 2600);
  }, []);

  const act = useCallback(async <R,>(fn: () => Promise<R>, okText?: string) => {
    try {
      const result = await fn();
      if (okText) notify(okText, "ok");
      await refresh();
      return result;
    } catch (err) {
      notify(t.msg(errorMsg(err)), "err");
      await refresh();
      return null;
    }
  }, [notify, refresh, t]);

  // Shared by the button in Settings and the same check at startup, so either shows the other's progress.
  const [update, setUpdate] = useState<string | null>(null);
  const checkUpdate = useCallback(async (quiet = false) => {
    setUpdate("");
    try {
      const version = await ipc.checkUpdate();
      if (!version) {
        if (!quiet) notify(t("settings.up_to_date"), "ok");
        setUpdate(null);
        return;
      }
      setUpdate(version);
      if (quiet) notify(t("update.auto", { version }), "ok");
      await ipc.installUpdate();
    } catch (err) {
      if (!quiet) notify(t.msg(errorMsg(err)), "err");
      setUpdate(null);
    }
  }, [notify, t]);

  const clearLogs = useCallback(() => setLogs([]), []);

  useEffect(() => {
    void refresh();
    return onState(() => { void refresh(); });
  }, [refresh]);

  // The tray menu's window never shows these.
  useEffect(() => {
    if (!withLogs) return;
    const numbered = (line: LogLine): Line => ({ ...line, id: ++lineId.current });
    void ipc.logs().then((lines) => setLogs(lines.map(numbered))).catch(() => undefined);
    const offLog = onLog((line) => setLogs((list) => [...list, numbered(line)].slice(-MAX_LOGS)));
    const offProgress = onProgress(setProgress);
    return () => { offLog(); offProgress(); };
  }, [withLogs]);

  const tRef = useRef(t);
  tRef.current = t;
  useEffect(() => {
    if (!withLogs) return;
    return onNotice((n) => notify(tRef.current.msg(n.msg), n.kind));
  }, [withLogs, notify]);

  const store = useMemo<Store>(
    () => ({ snapshot, t, toasts, progress, refresh, notify, act, update, checkUpdate }),
    [snapshot, t, toasts, progress, refresh, notify, act, update, checkUpdate],
  );
  const logsValue = useMemo<Logs>(() => ({ logs, clearLogs }), [logs, clearLogs]);
  return (
    <Ctx.Provider value={store}>
      <LogsCtx.Provider value={logsValue}>{children}</LogsCtx.Provider>
    </Ctx.Provider>
  );
}
