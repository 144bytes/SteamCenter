import { useStore } from "../state/store";
import { ipc } from "../api/ipc";

/** The tray icon's menu: its own small borderless window (app/src/platform/tray.rs), drawn in the app's style. */
export function TrayMenu() {
  const { t } = useStore();
  return (
    <div className="tray-menu">
      <button type="button" className="nav-item" onClick={() => void ipc.openWindow()}>{t("tray.open")}</button>
      <button type="button" className="nav-item" onClick={() => void ipc.quit(false)}>{t("tray.quit")}</button>
    </div>
  );
}
