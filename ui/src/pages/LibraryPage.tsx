import { useMemo, useState } from "react";
import { useStore } from "../state/store";
import { ipc } from "../api/ipc";
import {
  ActionButton, Confirm, ItemList, ItemRow, Meta, MetaRow, PageHead, SectionHead, Select, Status, TextInput,
} from "../components/ui";
import { LoadError, SignedIn } from "../components/SignIn";
import { RulesModal } from "../components/modals";
import { useLoad } from "../hooks/useLoad";
import type { Game } from "../api/types";

const FILTERS = ["all", "private", "public", "unplayed"] as const;
type Filter = (typeof FILTERS)[number];

/** The question before a change, and the change itself on its confirmation. */
interface Ask { title: string; text: string; action: string; run: () => Promise<unknown> }

export function LibraryPage() {
  const { t } = useStore();
  return (
    <>
      <PageHead title={t("library.title")} sub={t("library.sub")} />
      <SignedIn><Library /></SignedIn>
    </>
  );
}

function Library() {
  const { snapshot, t, act, notify, progress } = useStore();
  const key = `${snapshot!.account!.steamid}/${snapshot!.settings.language}`;
  const games = useLoad(ipc.library, key);
  const backups = useLoad(ipc.backups, key);
  const [filter, setFilter] = useState<Filter>("all");
  const [search, setSearch] = useState("");
  const [selected, setSelected] = useState<Set<number>>(new Set());
  const [ask, setAsk] = useState<Ask | null>(null);
  const [rules, setRules] = useState(false);
  const busy = snapshot!.job !== null;

  const all = games.data ?? [];
  const visible = useMemo(() => {
    const q = search.trim().toLowerCase();
    return all.filter((g) => (filter === "all" || (filter === "private" ? g.private : filter === "public" ? !g.private : g.playtime === 0))
      && (!q || g.name.toLowerCase().includes(q) || String(g.appid) === q));
  }, [all, filter, search]);
  const picked = visible.filter((g) => selected.has(g.appid));

  /** Change the mark of `list`, then show the library as Steam now has it. */
  const change = (list: Game[], makePrivate: boolean) => act(async () => {
    const n = await ipc.setPrivate(list.map((g) => g.appid), makePrivate);
    setSelected(new Set());
    await Promise.all([games.reload(), backups.reload()]);
    return n;
  }).then((n) => n !== null && notifyChanged(n));
  const notifyChanged = (n: number) => notify(n ? t("library.changed", { n }) : t("library.nothing_changed"), "ok");

  const askChange = (list: Game[], makePrivate: boolean) => setAsk({
    title: t(makePrivate ? "library.hide_title" : "library.show_title"),
    text: t(makePrivate ? "library.hide_confirm" : "library.show_confirm", { n: list.length }),
    action: t(makePrivate ? "library.hide" : "library.show"),
    run: () => change(list, makePrivate),
  });

  const restore = (name: string, created: number) => setAsk({
    title: t("library.restore_title"),
    text: t("library.restore_confirm", { date: t.time(created) }),
    action: t("library.restore"),
    run: () => act(async () => {
      const n = await ipc.restoreBackup(name);
      await Promise.all([games.reload(), backups.reload()]);
      return n;
    }).then((n) => n !== null && notifyChanged(n)),
  });

  const toggle = (appid: number, on: boolean) => setSelected((s) => {
    const next = new Set(s);
    if (on) next.add(appid); else next.delete(appid);
    return next;
  });
  const progressText = progress && snapshot!.job === "library" ? t("common.progress", { ...progress }) : null;
  const privateCount = all.filter((g) => g.private).length;

  if (games.error && !games.data) return <LoadError error={games.error} code={games.errorCode} onRetry={() => void games.reload()} />;
  if (!games.data) return <div className="muted icon-line"><span className="spinner" />{t("common.loading")}</div>;

  return (
    <>
      <div className="card">
        <MetaRow>
          <Meta label={t("library.games")} value={String(all.length)} />
          <Meta label={t("library.private")} value={String(privateCount)} />
          <Meta label={t("library.public")} value={String(all.length - privateCount)} />
          <Meta label={t("library.playtime")} value={t.hours(all.reduce((sum, g) => sum + g.playtime, 0))} />
        </MetaRow>
      </div>

      <SectionHead title={t("library.games")} action={
        <ActionButton disabled={games.loading} onClick={() => void games.reload()}>
          {games.loading ? t("common.loading") : t("common.refresh")}
        </ActionButton>
      } />
      <div className="filters">
        <TextInput className="search" value={search} onChange={setSearch} placeholder={t("library.search")} spellCheck={false} />
        <Select value={filter} onChange={setFilter} title={t("library.filter")}
                options={FILTERS.map((f) => ({ value: f, label: t.dyn(`library.filter.${f}`) }))} />
        <ActionButton disabled={busy || !all.length} onClick={() => setRules(true)}>{t("library.rules")}</ActionButton>
        <span className="spacer" />
        <ActionButton variant="primary" disabled={busy || !picked.length} onClick={() => askChange(picked, true)}>
          {progressText ?? t("library.hide_selected")}
        </ActionButton>
        <ActionButton disabled={busy || !picked.length} onClick={() => askChange(picked, false)}>
          {t("library.show_selected")}
        </ActionButton>
      </div>
      <ItemList count={visible.length} countText={t("list.games", { n: visible.length })} selected={picked.length} disabled={busy}
                onSelectAll={(on) => setSelected(on ? new Set(visible.map((g) => g.appid)) : new Set())}>
        {visible.map((g) => (
          <ItemRow key={g.appid} checked={selected.has(g.appid)} onCheck={(on) => toggle(g.appid, on)} disabled={busy}
                   image={g.icon} name={g.name}
                   sub={g.playtime
                     ? t("library.played", { hours: t.hours(g.playtime), date: t.date(g.last_played) })
                     : t("library.never_played")}
                   status={g.private
                     ? <Status state="error" label={t("library.private_one")} />
                     : <Status state="connected" label={t("library.public_one")} />}
                   action={
                     <ActionButton variant="ghost" disabled={busy} onClick={() => void change([g], !g.private)}>
                       {g.private ? t("library.show") : t("library.hide")}
                     </ActionButton>
                   } />
        ))}
      </ItemList>

      <SectionHead title={t("library.backups")} />
      <div className="card">
        {backups.data?.length ? backups.data.map((b) => (
          <div key={b.name} className="backup-row">
            <span>{t.time(b.created)}</span>
            <span className="muted">{t("library.backup_count", { n: b.count })}</span>
            <ActionButton variant="ghost" disabled={busy} onClick={() => restore(b.name, b.created)}>
              {t("library.restore")}
            </ActionButton>
          </div>
        )) : <p className="faint backup-empty">{t("library.no_backups")}</p>}
      </div>

      {rules && <RulesModal games={all} onClose={() => setRules(false)} onApply={(list, makePrivate) => void change(list, makePrivate)} />}

      {ask && (
        <Confirm title={ask.title} text={ask.text} action={ask.action}
                 onCancel={() => setAsk(null)} onConfirm={() => { setAsk(null); void ask.run(); }} />
      )}
    </>
  );
}
