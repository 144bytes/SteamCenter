import { useStore } from "../state/store";
import { ipc } from "../api/ipc";
import {
  ActionButton, Meta, MetaRow, PageHead, SectionHead, Select, SettingRow, SettingsGroup,
} from "../components/ui";
import { LoadError, SignedIn } from "../components/SignIn";
import { useLoad } from "../hooks/useLoad";
import type { Appearance, ItemKind } from "../api/types";

export function ProfilePage() {
  const { t } = useStore();
  return (
    <>
      <PageHead title={t("profile.title")} sub={t("profile.sub")} />
      <SignedIn><Profile /></SignedIn>
    </>
  );
}

function Profile() {
  const { snapshot, t } = useStore();
  const key = `${snapshot!.account!.steamid}/${snapshot!.settings.language}`;
  const profile = useLoad(ipc.profile, key);

  if (profile.error && !profile.data) return <LoadError error={profile.error} code={profile.errorCode} onRetry={() => void profile.reload()} />;
  if (!profile.data) return <div className="muted icon-line"><span className="spinner" />{t("common.loading")}</div>;
  const { summary: s, level, recent } = profile.data;
  // The profile's own pages on the Steam site (the address ends with "/").
  const page = (path: string) => () => void ipc.openUrl(`${s.profile_url.replace(/\/?$/, "/")}${path}`);

  return (
    <>
      <div className="card profile-card">
        {s.avatar && <img className="avatar" src={s.avatar} alt="" />}
        <div className="profile-id">
          <div className="profile-name">{s.name}</div>
          <MetaRow>
            <Meta label={t("profile.level")} value={String(level)} />
            <Meta label={t("profile.visibility")} value={t.dyn(`profile.visibility.${s.visibility}`)} />
            <Meta label={t("profile.created")} value={t.date(s.created)} />
            <Meta label="SteamID" value={s.steamid} />
          </MetaRow>
        </div>
      </div>
      <div className="filters profile-links">
        <ActionButton onClick={page("")}>{t("profile.open")}</ActionButton>
        <ActionButton onClick={page("edit/info")}>{t("profile.edit")}</ActionButton>
        <ActionButton onClick={page("edit/settings")}>{t("profile.privacy")}</ActionButton>
      </div>

      <SectionHead title={t("profile.recent")} />
      <div className="card">
        {recent.length ? recent.map((g) => (
          <div key={g.appid} className="recent-row">
            <span className="item-pic">{g.icon && <img src={g.icon} alt="" loading="lazy" />}</span>
            <span className="item-name">{g.name}</span>
            <span className="muted">{t("profile.recent_time", { recent: t.hours(g.playtime_2weeks), total: t.hours(g.playtime) })}</span>
          </div>
        )) : <p className="faint backup-empty">{t("profile.no_recent")}</p>}
      </div>

      <Look steamKey={key} />
    </>
  );
}

/** The profile's look: one picker per kind of item, and the theme. */
function Look({ steamKey }: { steamKey: string }) {
  const { t, act } = useStore();
  const look = useLoad(ipc.appearance, steamKey);
  const apply = (fn: () => Promise<void>) => act(fn, t("profile.applied")).then(() => look.reload());

  return (
    <>
      <SectionHead title={t("profile.look")} action={
        <ActionButton disabled={look.loading} onClick={() => void look.reload()}>
          {look.loading ? t("common.loading") : t("common.refresh")}
        </ActionButton>
      } />
      {look.error && !look.data
        ? <LoadError error={look.error} code={look.errorCode} onRetry={() => void look.reload()} />
        : !look.data
          ? <div className="muted icon-line"><span className="spinner" />{t("common.loading")}</div>
          : <LookCard look={look.data} disabled={look.loading}
                      onItem={(kind, id) => void apply(() => ipc.setItem(kind, id))}
                      onTheme={(id) => void apply(() => ipc.setTheme(id))} />}
    </>
  );
}

function LookCard({ look, disabled, onItem, onTheme }: {
  look: Appearance;
  disabled: boolean;
  onItem: (kind: ItemKind, id: string) => void;
  onTheme: (id: string) => void;
}) {
  const { t } = useStore();
  return (
    <div className="card">
      <SettingsGroup>
        {look.slots.map((slot) => {
          const worn = slot.items.find((i) => i.id === slot.equipped);
          return (
            <SettingRow key={slot.kind} label={t.dyn(`profile.kind.${slot.kind}`)}
                        hint={t("profile.owned", { n: slot.items.length })}>
              <span className="thumb">{worn?.image && <img src={worn.image} alt="" />}</span>
              <Select value={slot.equipped} disabled={disabled || !slot.items.length}
                      onChange={(id) => onItem(slot.kind, id)}
                      options={[{ value: "", label: t("profile.none") },
                        ...slot.items.map((i) => ({ value: i.id, label: i.name || i.id }))]} />
            </SettingRow>
          );
        })}
        <SettingRow label={t("profile.theme")} hint={t("profile.theme_hint")}>
          <Select value={look.theme} disabled={disabled} onChange={onTheme}
                  options={[{ value: "", label: t("profile.theme_default") },
                    ...look.themes.filter((th) => th.id).map((th) => ({ value: th.id, label: th.title }))]} />
        </SettingRow>
      </SettingsGroup>
    </div>
  );
}
