import { useEffect, useRef, useState } from "react";
import { useStore } from "./state/store";
import { ipc, onQuitRequested } from "./api/ipc";
import { Confirm, Status } from "./components/ui";
import { LibraryPage } from "./pages/LibraryPage";
import { FriendsPage } from "./pages/FriendsPage";
import { ProfilePage } from "./pages/ProfilePage";
import { WishlistPage } from "./pages/WishlistPage";
import { SecurityPage } from "./pages/SecurityPage";
import { SettingsPage } from "./pages/SettingsPage";
import { DiagnosticsPage } from "./pages/DiagnosticsPage";
import appIcon from "./assets/app-icon.svg";

type Route = "library" | "friends" | "profile" | "wishlist" | "security" | "settings" | "diagnostics";

const ROUTES: { key: Route; icon: string }[] = [
  { key: "library", icon: "▦" },
  { key: "friends", icon: "☺" },
  { key: "profile", icon: "◉" },
  { key: "wishlist", icon: "★" },
  { key: "security", icon: "◈" },
  { key: "settings", icon: "⚙" },
  { key: "diagnostics", icon: "≡" },
];

function currentRoute(): Route {
  const raw = window.location.hash.replace(/^#\/?/, "").split("/")[0];
  return (ROUTES.some((r) => r.key === raw) ? raw : "library") as Route;
}

export function App() {
  const { snapshot, t, toasts, checkUpdate } = useStore();
  const [route, setRoute] = useState<Route>(currentRoute);
  const [askQuit, setAskQuit] = useState(false);

  useEffect(() => {
    const onHash = () => setRoute(currentRoute());
    window.addEventListener("hashchange", onHash);
    return () => window.removeEventListener("hashchange", onHash);
  }, []);
  useEffect(() => onQuitRequested(() => setAskQuit(true)), []);
  // "Check for updates" by itself, once per start, when the setting is on.
  const checked = useRef(false);
  useEffect(() => {
    if (!snapshot || checked.current) return;
    checked.current = true;
    if (snapshot.update_on_start && snapshot.settings.auto_update) void checkUpdate(true);
  }, [snapshot, checkUpdate]);

  if (!snapshot) {
    return (
      <div className="splash muted icon-line">
        <span className="spinner" />
        {t("common.loading")}
      </div>
    );
  }

  const account = snapshot.account;

  return (
    <div className="shell">
      <nav className="sidebar">
        <div className="brand">
          <img className="brand-mark" src={appIcon} alt="" />
          <span className="brand-name">{t("app.name")}</span>
        </div>
        {ROUTES.map((item) => (
          <button key={item.key} type="button" className={`nav-item${route === item.key ? " active" : ""}`}
                  onClick={() => { window.location.hash = `#/${item.key}`; }}>
            <span className="nav-icon">{item.icon}</span>
            <span>{t(`nav.${item.key}`)}</span>
          </button>
        ))}
        <div className="nav-spacer" />
        <div className="nav-foot">
          {snapshot.signing_in
            ? <Status state="connecting" label={t("account.signing_in")} />
            : !account ? <Status state="idle" label={t("account.signed_out")} />
            : account.expires ? <Status state="connected" label={account.name} />
            : <Status state="error" label={t("account.expired")} />}
        </div>
      </nav>

      <main className="main">
        {route === "library" && <LibraryPage />}
        {route === "friends" && <FriendsPage />}
        {route === "profile" && <ProfilePage />}
        {route === "wishlist" && <WishlistPage />}
        {route === "security" && <SecurityPage />}
        {route === "settings" && <SettingsPage />}
        {route === "diagnostics" && <DiagnosticsPage />}
      </main>

      {askQuit && (
        <Confirm title={t("quit.title")} text={t("quit.confirm")} action={t("tray.quit")}
                 onCancel={() => setAskQuit(false)} onConfirm={() => void ipc.quit(true)} />
      )}

      <div className="toasts">
        {toasts.map((toast) => (
          <div key={toast.id} className={`toast ${toast.kind}`}>{toast.text}</div>
        ))}
      </div>
    </div>
  );
}
