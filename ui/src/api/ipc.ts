/** Calls into the Rust side over Tauri IPC. Errors arrive as a message code (`Msg`) and are translated on screen. */
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import type { Msg } from "../i18n";
import type {
  Appearance, Backup, Game, ItemKind, LogLine, Person, Profile, Progress, Session, Settings, Snapshot, StoreTags,
  WishItem,
} from "./types";

class IpcError extends Error {
  constructor(public msg: Msg) {
    super(msg.code);
  }
}

/** An error that came without a code (for example, from Windows): its text, shown as is. */
const raw = (err: unknown): Msg => ({ code: "raw", params: { text: String(err) } });

async function call<R>(cmd: string, args?: Record<string, unknown>): Promise<R> {
  try {
    return await invoke<R>(cmd, args);
  } catch (err) {
    throw new IpcError(typeof err === "object" && err && "code" in err ? err as Msg : raw(err));
  }
}

export function errorMsg(err: unknown): Msg {
  return err instanceof IpcError ? err.msg : raw(err);
}

export const ipc = {
  state: () => call<Snapshot>("get_state"),
  saveSettings: (settings: Settings) => call<void>("save_settings", { settings }),
  logs: () => call<LogLine[]>("get_logs"),
  openLogs: () => call<void>("open_logs"),
  /** `filter` - the file type name in the save dialog. */
  saveReport: (filter: string) => call<string | null>("save_report", { filter }),
  /** A Steam page in the default browser. */
  openUrl: (url: string) => call<void>("open_url", { url }),
  openWindow: () => call<void>("open_window"),
  /** `force` - the user already confirmed quitting while an operation runs. */
  quit: (force: boolean) => call<void>("quit", { force }),

  signIn: () => call<void>("sign_in"),
  signInToken: (token: string) => call<void>("sign_in_token", { token }),
  signOut: () => call<void>("sign_out"),

  library: () => call<Game[]>("get_library"),
  /** Returns how many games changed. */
  setPrivate: (appids: number[], isPrivate: boolean) => call<number>("set_private", { appids, private: isPrivate }),
  storeTags: (appids: number[]) => call<StoreTags>("get_store_tags", { appids }),
  backups: () => call<Backup[]>("list_backups"),
  restoreBackup: (name: string) => call<number>("restore_backup", { name }),

  profile: () => call<Profile>("get_profile"),
  appearance: () => call<Appearance>("get_appearance"),
  /** An empty `id` takes the worn item off. */
  setItem: (kind: ItemKind, id: string) => call<void>("set_item", { kind, id }),
  setTheme: (id: string) => call<void>("set_theme", { id }),

  friends: () => call<Person[]>("get_friends"),
  /** Returns how many were removed. */
  removeFriends: (steamids: string[]) => call<number>("remove_friends", { steamids }),
  sessions: () => call<Session[]>("get_sessions"),

  wishlist: () => call<WishItem[]>("get_wishlist"),
  removeWishlist: (appids: number[]) => call<number>("remove_wishlist", { appids }),

  checkUpdate: () => call<string | null>("check_update"),
  installUpdate: () => call<void>("install_update"),
};

/** Subscription to a Rust-side event; returns the unsubscribe. */
function on<P>(event: string, handler: (payload: P) => void): () => void {
  const p = listen<P>(event, (e) => handler(e.payload));
  return () => { void p.then((un) => un()); };
}

/** "state" - data changed, "log" - a new log line. */
export const onState = (handler: () => void) => on("state", () => handler());
export const onLog = (handler: (line: LogLine) => void) => on("log", handler);
/** A message the Rust side shows by itself (the sign-in finished or failed). */
export const onNotice = (handler: (n: { kind: "ok" | "err"; msg: Msg }) => void) => on("notice", handler);
/** The running operation's progress; null - it ended. */
export const onProgress = (handler: (p: Progress | null) => void) => on("progress", handler);
/** "Quit" from the tray while an operation runs: the main window asks first. */
export const onQuitRequested = (handler: () => void) => on("quit-requested", () => handler());
