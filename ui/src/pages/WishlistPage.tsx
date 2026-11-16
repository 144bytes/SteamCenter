import { useMemo, useState } from "react";
import { useStore } from "../state/store";
import { ipc } from "../api/ipc";
import { ActionButton, Confirm, ItemList, ItemRow, PageHead, SectionHead, TextInput } from "../components/ui";
import { LoadError, SignedIn } from "../components/SignIn";
import { useLoad } from "../hooks/useLoad";

export function WishlistPage() {
  const { t } = useStore();
  return (
    <>
      <PageHead title={t("wishlist.title")} sub={t("wishlist.sub")} />
      <SignedIn><Wishlist /></SignedIn>
    </>
  );
}

function Wishlist() {
  const { snapshot, t, act, notify, progress } = useStore();
  const wishlist = useLoad(ipc.wishlist, `${snapshot!.account!.steamid}/${snapshot!.settings.language}`);
  const [search, setSearch] = useState("");
  const [selected, setSelected] = useState<Set<number>>(new Set());
  const [ask, setAsk] = useState<number[] | null>(null);
  const busy = snapshot!.job !== null;

  const all = wishlist.data ?? [];
  const visible = useMemo(() => {
    const q = search.trim().toLowerCase();
    return all.filter((w) => !q || w.name.toLowerCase().includes(q) || String(w.appid) === q);
  }, [all, search]);
  const picked = visible.filter((w) => selected.has(w.appid)).map((w) => w.appid);

  const remove = (appids: number[]) => act(async () => {
    const n = await ipc.removeWishlist(appids);
    setSelected(new Set());
    await wishlist.reload();
    return n;
  }).then((n) => n !== null && notify(t("wishlist.removed", { n }), "ok"));

  const toggle = (appid: number, on: boolean) => setSelected((s) => {
    const next = new Set(s);
    if (on) next.add(appid); else next.delete(appid);
    return next;
  });

  if (wishlist.error && !wishlist.data) return <LoadError error={wishlist.error} code={wishlist.errorCode} onRetry={() => void wishlist.reload()} />;
  if (!wishlist.data) return <div className="muted icon-line"><span className="spinner" />{t("common.loading")}</div>;

  return (
    <>
      <SectionHead title={t("wishlist.games", { n: all.length })} action={
        <ActionButton disabled={wishlist.loading} onClick={() => void wishlist.reload()}>
          {wishlist.loading ? t("common.loading") : t("common.refresh")}
        </ActionButton>
      } />
      <div className="filters">
        <TextInput className="search" value={search} onChange={setSearch} placeholder={t("library.search")} spellCheck={false} />
        <span className="spacer" />
        <ActionButton variant="danger" disabled={busy || !picked.length} onClick={() => setAsk(picked)}>
          {progress && snapshot!.job === "wishlist" ? t("common.progress", { ...progress }) : t("wishlist.remove_selected")}
        </ActionButton>
      </div>
      <ItemList count={visible.length} countText={t("list.games", { n: visible.length })} selected={picked.length} disabled={busy}
                onSelectAll={(on) => setSelected(on ? new Set(visible.map((w) => w.appid)) : new Set())}>
        {visible.map((w) => (
          <ItemRow key={w.appid} checked={selected.has(w.appid)} onCheck={(on) => toggle(w.appid, on)} disabled={busy}
                   image={w.image} wide name={w.name} sub={t("wishlist.added", { date: t.date(w.added) })}
                   action={
                     <ActionButton variant="ghost-danger" disabled={busy} onClick={() => void remove([w.appid])}>
                       {t("wishlist.remove")}
                     </ActionButton>
                   } />
        ))}
      </ItemList>

      {ask && (
        <Confirm title={t("wishlist.remove_title")} text={t("wishlist.remove_confirm", { n: ask.length })}
                 action={t("wishlist.remove")} danger
                 onCancel={() => setAsk(null)} onConfirm={() => { const list = ask; setAsk(null); void remove(list); }} />
      )}
    </>
  );
}
