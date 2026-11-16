import { useMemo, useState } from "react";
import { useStore } from "../state/store";
import { ipc } from "../api/ipc";
import {
  ActionButton, Confirm, ItemList, ItemRow, Meta, MetaRow, PageHead, SectionHead, Select, Status, TextInput,
} from "../components/ui";
import { LoadError, SignedIn } from "../components/SignIn";
import { useLoad } from "../hooks/useLoad";
import type { Person } from "../api/types";

/** "Offline longer than": days. */
const AWAY = ["all", "30", "90", "180", "365", "730"] as const;
type Away = (typeof AWAY)[number];
const DAY = 86400;

export function FriendsPage() {
  const { t } = useStore();
  return (
    <>
      <PageHead title={t("friends.title")} sub={t("friends.sub")} />
      <SignedIn><Friends /></SignedIn>
    </>
  );
}

function Friends() {
  const { snapshot, t, act, notify, progress } = useStore();
  const friends = useLoad(ipc.friends, `${snapshot!.account!.steamid}/${snapshot!.settings.language}`);
  const [search, setSearch] = useState("");
  const [away, setAway] = useState<Away>("all");
  const [selected, setSelected] = useState<Set<string>>(new Set());
  const [ask, setAsk] = useState<Person[] | null>(null);
  const busy = snapshot!.job !== null;
  // "Now" of this visit: the list doesn't reshuffle while it is being looked at.
  const [now] = useState(() => Date.now() / 1000);

  const all = friends.data ?? [];
  const visible = useMemo(() => {
    const q = search.trim().toLowerCase();
    const list = all.filter((f) => (away === "all" || (!f.online && f.last_online > 0 && now - f.last_online > Number(away) * DAY))
      && (!q || f.name.toLowerCase().includes(q) || f.steamid === q));
    // Looking for the long gone: the longest away first.
    return away === "all" ? list : [...list].sort((a, b) => a.last_online - b.last_online);
  }, [all, away, search, now]);
  const picked = visible.filter((f) => selected.has(f.steamid));

  const remove = (list: Person[]) => act(async () => {
    const n = await ipc.removeFriends(list.map((f) => f.steamid));
    setSelected(new Set());
    await friends.reload();
    return n;
  }).then((n) => n !== null && notify(t("friends.removed", { n }), "ok"));

  const toggle = (id: string, on: boolean) => setSelected((s) => {
    const next = new Set(s);
    if (on) next.add(id); else next.delete(id);
    return next;
  });
  /** Days since last online, or why there is no number. */
  const seen = (f: Person) => f.online ? t("friends.online")
    : f.last_online ? t("friends.last_online", { date: t.date(f.last_online) })
    : t("friends.unknown");

  if (friends.error && !friends.data) return <LoadError error={friends.error} code={friends.errorCode} onRetry={() => void friends.reload()} />;
  if (!friends.data) return <div className="muted icon-line"><span className="spinner" />{t("common.loading")}</div>;

  const online = all.filter((f) => f.online).length;
  const longAway = all.filter((f) => !f.online && f.last_online > 0 && now - f.last_online > 365 * DAY).length;

  return (
    <>
      <div className="card">
        <MetaRow>
          <Meta label={t("friends.total")} value={String(all.length)} />
          <Meta label={t("friends.online_now")} value={String(online)} />
          <Meta label={t("friends.away_year")} value={String(longAway)} />
          <Meta label={t("friends.unknown_count")} value={String(all.filter((f) => !f.online && !f.last_online).length)} />
        </MetaRow>
      </div>

      <SectionHead title={t("friends.title")} action={
        <ActionButton disabled={friends.loading} onClick={() => void friends.reload()}>
          {friends.loading ? t("common.loading") : t("common.refresh")}
        </ActionButton>
      } />
      <div className="filters">
        <TextInput className="search" value={search} onChange={setSearch} placeholder={t("friends.search")} spellCheck={false} />
        <Select value={away} onChange={setAway} title={t("friends.away")}
                options={AWAY.map((a) => ({ value: a, label: t.dyn(`friends.away.${a}`) }))} />
        <span className="spacer" />
        <ActionButton variant="danger" disabled={busy || !picked.length} onClick={() => setAsk(picked)}>
          {progress && snapshot!.job === "friends" ? t("common.progress", { ...progress }) : t("friends.remove_selected")}
        </ActionButton>
      </div>
      <ItemList count={visible.length} countText={t("friends.count", { n: visible.length })} selected={picked.length} disabled={busy}
                onSelectAll={(on) => setSelected(on ? new Set(visible.map((f) => f.steamid)) : new Set())}>
        {visible.map((f) => (
          <ItemRow key={f.steamid} checked={selected.has(f.steamid)} onCheck={(on) => toggle(f.steamid, on)} disabled={busy}
                   image={f.avatar} name={f.name || f.steamid} sub={f.steamid}
                   status={<Status state={f.online ? "connected" : "idle"} label={seen(f)} />}
                   action={
                     <ActionButton variant="ghost-danger" disabled={busy} onClick={() => setAsk([f])}>
                       {t("friends.remove")}
                     </ActionButton>
                   } />
        ))}
      </ItemList>

      {ask && (
        <Confirm title={t("friends.remove_title")} danger action={t("friends.remove")}
                 text={ask.length === 1 ? t("friends.remove_one", { name: ask[0].name }) : t("friends.remove_confirm", { n: ask.length })}
                 onCancel={() => setAsk(null)} onConfirm={() => { const list = ask; setAsk(null); void remove(list); }} />
      )}
    </>
  );
}
