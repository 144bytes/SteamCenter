import type { ReactNode } from "react";

export function SettingsGroup({ title, children }: { title?: string; children: ReactNode }) {
  return (
    <div className="settings-group">
      {title && <p className="settings-group-title">{title}</p>}
      {children}
    </div>
  );
}

/** One setting: what it is on the left, its control on the right. */
export function SettingRow({ label, hint, children }: {
  label: string; hint?: string; children: ReactNode;
}) {
  return (
    <div className="setting-row">
      <div>
        <div className="setting-label">{label}</div>
        {hint && <div className="setting-hint">{hint}</div>}
      </div>
      <div className="setting-control">{children}</div>
    </div>
  );
}
