import { useMemo, useState } from "react";
import { ipc } from "../../api/ipc";
import { Field, FormError, FormFooter, Modal, Select } from "../ui";
import { useLoad } from "../../hooks/useLoad";
import { useStore } from "../../state/store";
import type { Game } from "../../api/types";

const RULES = ["adult", "unplayed", "short", "free", "tag"] as const;
type Rule = (typeof RULES)[number];
/** Rules that need the store's data about the games. */
const STORE_RULES: readonly Rule[] = ["adult", "free", "tag"];
/** "Played less than an hour". */
const SHORT_MINUTES = 60;

/** Make private (or public) every game a rule picks: a preset or a store tag. Shows how many games it picks and
 *  which, before anything changes; the library page makes the change (and its backup). */
export function RulesModal({ games, onApply, onClose }: {
  games: Game[];
  onApply: (list: Game[], makePrivate: boolean) => void;
  onClose: () => void;
}) {
  const { snapshot, t } = useStore();
  const [rule, setRule] = useState<Rule>("adult");
  const [tag, setTag] = useState("");
  const [action, setAction] = useState<"hide" | "show">("hide");
  // The store's data is loaded once, when the window opens.
  const store = useLoad(() => ipc.storeTags(games.map((g) => g.appid)), snapshot!.account!.steamid);
  const needsStore = STORE_RULES.includes(rule);
  const tags = store.data?.tags ?? [];
  const tagId = Number(tag || tags[0]?.id || 0);

  const matched = useMemo(() => games.filter((g) => {
    const s = store.data?.games[g.appid];
    switch (rule) {
      case "adult": return !!s?.adult;
      case "unplayed": return g.playtime === 0;
      case "short": return g.playtime < SHORT_MINUTES;
      case "free": return !!s?.free;
      case "tag": return !!s?.tags.includes(tagId);
    }
  }), [games, rule, store.data, tagId]);
  const makePrivate = action === "hide";
  const changes = matched.filter((g) => g.private !== makePrivate);
  const waiting = needsStore && !store.data;

  return (
    <Modal title={t("rules.title")} onClose={onClose} footer={
      <FormFooter onCancel={onClose} busy={waiting || !changes.length}
                  label={t(makePrivate ? "rules.hide" : "rules.show", { n: changes.length })}
                  onConfirm={() => { onClose(); onApply(changes, makePrivate); }} />
    }>
      <Field label={t("rules.rule")}>
        <Select value={rule} onChange={setRule} options={RULES.map((r) => ({ value: r, label: t.dyn(`rules.rule.${r}`) }))} />
      </Field>
      {rule === "tag" && (
        <Field label={t("rules.tag")} hint={t("rules.tag_hint")}>
          <Select value={String(tagId)} onChange={setTag} disabled={!tags.length}
                  options={tags.map((x) => ({ value: String(x.id), label: `${x.name} (${x.count})` }))} />
        </Field>
      )}
      <Field label={t("rules.action")}>
        <Select value={action} onChange={setAction}
                options={[{ value: "hide", label: t("rules.action.hide") }, { value: "show", label: t("rules.action.show") }]} />
      </Field>
      {needsStore && store.error && !store.data && <FormError message={store.error} />}
      {waiting && !store.error && <p className="muted icon-line"><span className="spinner" />{t("rules.loading")}</p>}
      {!waiting && (
        <div className="field">
          <p className="muted rules-count">{t("rules.matched", { n: matched.length, changes: changes.length })}</p>
          {changes.length > 0 && (
            <ul className="rules-preview">
              {changes.slice(0, 8).map((g) => <li key={g.appid}>{g.name}</li>)}
              {changes.length > 8 && <li className="faint">{t("rules.more", { n: changes.length - 8 })}</li>}
            </ul>
          )}
        </div>
      )}
    </Modal>
  );
}
