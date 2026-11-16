import { useStore } from "../state/store";
import { ipc } from "../api/ipc";
import { ActionButton, PageHead, SectionHead, Status } from "../components/ui";
import { LoadError, SignedIn } from "../components/SignIn";
import { useLoad } from "../hooks/useLoad";
import type { Seen } from "../api/types";

/** Steam's own page for signing devices out: revoking needs a signature only Steam can make. */
const DEVICES_PAGE = "https://store.steampowered.com/account/authorizeddevices";

export function SecurityPage() {
  const { t } = useStore();
  return (
    <>
      <PageHead title={t("security.title")} sub={t("security.sub")} actions={
        <ActionButton className="wide" onClick={() => void ipc.openUrl(DEVICES_PAGE)}>{t("security.manage")}</ActionButton>
      } />
      <SignedIn><Sessions /></SignedIn>
    </>
  );
}

function Sessions() {
  const { snapshot, t } = useStore();
  const sessions = useLoad(ipc.sessions, `${snapshot!.account!.steamid}/${snapshot!.settings.language}`);

  if (sessions.error && !sessions.data) return <LoadError error={sessions.error} code={sessions.errorCode} onRetry={() => void sessions.reload()} />;
  if (!sessions.data) return <div className="muted icon-line"><span className="spinner" />{t("common.loading")}</div>;

  /** When, where from (place, IP). */
  const where = (s: Seen | null) => s
    ? [s.time ? t.time(s.time) : "", s.place, s.ip].filter(Boolean).join(" · ")
    : "-";

  return (
    <>
      <SectionHead title={t("security.sessions", { n: sessions.data.length })} action={
        <ActionButton disabled={sessions.loading} onClick={() => void sessions.reload()}>
          {sessions.loading ? t("common.loading") : t("common.refresh")}
        </ActionButton>
      } />
      <div className="card">
        {sessions.data.length ? sessions.data.map((s) => (
          <div key={s.id} className="session-row">
            <div className="item-id">
              <div className="item-name" title={s.description}>
                {t.dyn(`security.platform.${s.platform}`)}{s.description && ` · ${s.description}`}
              </div>
              <div className="item-sub">{t("security.first_seen", { when: where(s.first_seen) })}</div>
            </div>
            <div className="item-id">
              <div className="meta-label">{t("security.last_seen")}</div>
              <div className="meta-value" title={where(s.last_seen)}>{where(s.last_seen)}</div>
            </div>
            <span className="item-status">
              {s.current ? <Status state="connected" label={t("security.current")} />
                : <Status state={s.logged_in ? "connected" : "idle"} label={t(s.logged_in ? "security.logged_in" : "security.remembered")} />}
            </span>
          </div>
        )) : <p className="faint backup-empty">{t("security.none")}</p>}
      </div>
    </>
  );
}
