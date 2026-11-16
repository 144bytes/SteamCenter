/** Translation of the interface: `t(key)` for the texts, `t.msg` for the messages that come from the Rust side.
 *  English is the reference language; the texts themselves are in `locales/`.
 */
import { en } from "./locales/en";
import { ru } from "./locales/ru";

export type Lang = "en" | "ru";

export const LANGS: readonly Lang[] = ["en", "ru"];

type Key = keyof typeof en;

/** A message from the Rust side: a code and what to substitute into the text. */
export interface Msg {
  code: string;
  params?: Record<string, unknown>;
}

export function makeT(lang: Lang) {
  const dict: Record<string, string> = lang === "ru" ? ru : en;

  /** A parameter may be a message itself: it is translated too. */
  function fill(text: string, params?: Record<string, unknown>): string {
    if (!params) return text;
    return text.replace(/\{(\w+)\}/g, (_all, name: string) => {
      const value = params[name];
      if (value === undefined || value === null) return "";
      return typeof value === "object" && "code" in value ? t.msg(value as Msg) : String(value);
    });
  }

  /** A key known when writing the code: checked by the compiler. */
  function t(key: Key, params?: Record<string, unknown>): string {
    return fill(dict[key], params);
  }

  /** A key built at runtime (state, kind). */
  t.dyn = function dyn(key: string, params?: Record<string, unknown>): string {
    return fill(dict[key] ?? key, params);
  };

  /** A message from the Rust side. "raw" - text from outside (for example, from Windows), shown as is. */
  t.msg = function translate(message: Msg): string {
    if (message.code === "raw") return String(message.params?.text ?? "");
    return t.dyn(message.code, message.params);
  };

  /** Play time (minutes) in hours, one decimal, in the language's format. */
  t.hours = function hours(minutes: number): string {
    const h = (minutes / 60).toLocaleString(lang, { minimumFractionDigits: 1, maximumFractionDigits: 1 });
    return t("unit.hours", { h });
  };

  /** A date (unix seconds); 0 - unknown. */
  t.date = function date(unix: number): string {
    return unix ? new Date(unix * 1000).toLocaleDateString(lang, { year: "numeric", month: "short", day: "numeric" }) : "-";
  };

  /** A date and time (unix seconds). */
  t.time = function time(unix: number): string {
    return new Date(unix * 1000).toLocaleString(lang, { dateStyle: "medium", timeStyle: "short" });
  };

  return t;
}

export type T = ReturnType<typeof makeT>;
