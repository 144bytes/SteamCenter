import { useState } from "react";
import { useStore } from "../state/store";
import { ipc } from "../api/ipc";
import { ActionButton, Confirm, PageHead, Select, SettingRow, SettingsGroup, Switch, Tabs } from "../components/ui";
import { SignInButtons } from "../components/SignIn";
import { useSaveSettings } from "../hooks/useSaveSettings";
import { LANGS } from "../i18n";

const SECTIONS = ["account", "general"] as const;
type Section = (typeof SECTIONS)[number];

export function SettingsPage() {
  const { snapshot, t } = useStore();
  const [section, setSection] = useState<Section>("account");
  if (!snapshot) return null;

  return (
    <>
      <PageHead title={t("settings.title")} sub={t("settings.sub")} />
      <div className="settings-layout">
        <Tabs value={section} onChange={setSection}
              items={SECTIONS.map((key) => ({ key, label: t(`settings.${key}`) }))} />
        <div className="stack">
          {section === "account" && <AccountSettings />}
          {section === "general" && <General />}
        </div>
      </div>
    </>
  );
}

function AccountSettings() {
  const { snapshot, t, act } = useStore();
  const [askOut, setAskOut] = useState(false);
  const a = snapshot!.account;
  if (!a) {
    return (
      <div className="card">
        <SettingsGroup>
          <SettingRow label={t("account.sign_in_title")} hint={t("account.sign_in_text")}>
            <span className="setting-buttons"><SignInButtons /></span>
          </SettingRow>
        </SettingsGroup>
      </div>
    );
  }
  return (
    <div className="card">
      <SettingsGroup>
        <SettingRow label={a.name} hint={`SteamID ${a.steamid}`}>
          <ActionButton variant="danger" disabled={snapshot!.job !== null} onClick={() => setAskOut(true)}>
            {t("account.sign_out")}
          </ActionButton>
        </SettingRow>
        <SettingRow label={t("account.session")}
                    hint={a.expires ? t("account.session_until", { date: t.time(a.expires) }) : t("account.expired")}>
          <span className="setting-buttons"><SignInButtons /></span>
        </SettingRow>
      </SettingsGroup>
      {askOut && (
        <Confirm title={t("account.sign_out_title")} text={t("account.sign_out_confirm")} action={t("account.sign_out")} danger
                 onCancel={() => setAskOut(false)}
                 onConfirm={() => { setAskOut(false); void act(() => ipc.signOut(), t("account.signed_out_ok")); }} />
      )}
    </div>
  );
}

function General() {
  const { snapshot, t, update, checkUpdate } = useStore();
  const save = useSaveSettings();
  return (
    <div className="card">
      <SettingsGroup>
        <SettingRow label={t("settings.language")}>
          <Select value={snapshot!.settings.language}
                  onChange={(v) => save((s) => { s.language = v; })}
                  options={LANGS.map((value) => ({ value, label: t.dyn(`lang.${value}`) }))} />
        </SettingRow>
        <SettingRow label={t("settings.close_to_tray")} hint={t("settings.close_to_tray_hint")}>
          <Switch checked={snapshot!.settings.close_to_tray} onChange={(v) => save((s) => { s.close_to_tray = v; })} />
        </SettingRow>
        <SettingRow label={t("settings.auto_update")} hint={t("settings.auto_update_hint")}>
          <Switch checked={snapshot!.settings.auto_update} onChange={(v) => save((s) => { s.auto_update = v; })} />
        </SettingRow>
        <SettingRow label={t("settings.version")} hint={snapshot!.version}>
          <ActionButton className="wide" disabled={update !== null} onClick={() => void checkUpdate()}>
            {update === null ? t("settings.check_update")
              : update === "" ? t("settings.checking")
              : t("settings.updating", { version: update })}
          </ActionButton>
        </SettingRow>
      </SettingsGroup>
    </div>
  );
}
