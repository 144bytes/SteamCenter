import type { ReactNode } from "react";

/** Label over value, in a card. */
export function Meta({ label, value }: { label: string; value: string }) {
  return (
    <div>
      <div className="meta-label">{label}</div>
      <div className="meta-value" title={value}>{value}</div>
    </div>
  );
}

export function MetaRow({ children }: { children: ReactNode }) {
  return <div className="meta-row">{children}</div>;
}
