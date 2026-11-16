/** Data provided by the app's Rust side (over Tauri IPC, no HTTP). Times are unix seconds, play time is minutes. */

import type { Lang } from "../i18n";

/** States drawn by `Status`: the Steam sign-in. */
export type State = "idle" | "connecting" | "connected" | "error";

export interface Settings {
  language: Lang;
  close_to_tray: boolean;
  auto_update: boolean;
  /** Card idling leaves out games with Valve Anti-Cheat. */
  skip_vac: boolean;
}

export interface CardGame {
  appid: number;
  name: string;
  /** Card drops left. */
  cards: number;
  hours: number;
  vac: boolean;
  image: string;
}

/** Idling for trading cards ("turbo"). */
export interface Idle {
  running: boolean;
  phase: "" | "loading" | "together" | "pause" | "solo";
  /** When the phase ends; 0 - no set end. */
  phase_ends: number;
  /** The games running now. */
  active: number[];
  /** The games with drops left, as last read. */
  games: CardGame[];
  loaded: boolean;
  cycle: number;
}

export interface Account {
  steamid: string;
  name: string;
  avatar: string;
  /** When the sign-in expires; 0 - it has to be renewed. */
  expires: number;
}

export interface Snapshot {
  version: string;
  settings: Settings;
  account: Account | null;
  /** The Steam sign-in window is open (or the sign-in is being renewed unseen). */
  signing_in: boolean;
  /** An operation changing the account is running. */
  job: "library" | "wishlist" | "friends" | null;
  idle: Idle;
  /** This start may check for updates by itself (not right after an update, not a debug build). */
  update_on_start: boolean;
}

export interface LogLine {
  ts: string;
  level: "INFO" | "WARNING" | "ERROR";
  module: string;
  message: string;
}

export interface Progress { done: number; total: number }

export interface Game {
  appid: number;
  name: string;
  icon: string;
  playtime: number;
  last_played: number;
  private: boolean;
}

export interface Backup { name: string; created: number; count: number }

/** The store's tags of the library's games, for "make private by rule". */
export interface StoreTags {
  /** By appid; games the store no longer sells are missing. */
  games: Record<string, { tags: number[]; adult: boolean; free: boolean }>;
  /** Most common first. */
  tags: { id: number; name: string; count: number }[];
}

/** A Steam profile: one's own or a friend's. */
export interface Person {
  steamid: string;
  name: string;
  profile_url: string;
  avatar: string;
  visibility: "public" | "friends" | "private";
  created: number;
  online: boolean;
  /** When last seen online; 0 - Steam doesn't say. */
  last_online: number;
}

export interface Session {
  id: string;
  description: string;
  platform: "client" | "browser" | "mobile" | "unknown";
  logged_in: boolean;
  /** The session this program signed in with. */
  current: boolean;
  first_seen: Seen | null;
  last_seen: Seen | null;
}

export interface Seen { time: number; ip: string; place: string }

export interface Profile {
  summary: Person;
  level: number;
  recent: { appid: number; name: string; icon: string; playtime_2weeks: number; playtime: number }[];
}

export type ItemKind = "background" | "mini_background" | "avatar_frame" | "animated_avatar";

export interface Appearance {
  slots: {
    kind: ItemKind;
    /** The worn item's id; empty - none. */
    equipped: string;
    items: { id: string; name: string; image: string; appid: number }[];
  }[];
  themes: { id: string; title: string }[];
  /** Empty - the default theme. */
  theme: string;
}

export interface WishItem {
  appid: number;
  name: string;
  image: string;
  added: number;
  priority: number;
}
