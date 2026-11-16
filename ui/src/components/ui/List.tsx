import { useState } from "react";
import type { ReactNode } from "react";
import { Check } from "./Fields";
import { useStore } from "../../state/store";

/** A list (games, friends) in one card: a head with "select all" and the column of actions, then the rows.
 *  `count` - rows shown; `countText` - how the head says it ("Games: 12"); `selected` - how many are ticked. */
export function ItemList({ count, countText, selected, onSelectAll, disabled, children }: {
  count: number;
  countText: string;
  selected: number;
  onSelectAll: (all: boolean) => void;
  disabled?: boolean;
  children: ReactNode;
}) {
  const { t } = useStore();
  return (
    <div className="item-list">
      <div className="item-list-head">
        <Check checked={count > 0 && selected === count} mixed={selected > 0 && selected < count}
               disabled={disabled || !count} title={t("list.select_all")} onChange={onSelectAll} />
        <span className="muted">{selected ? t("list.selected", { n: selected, total: count }) : countText}</span>
      </div>
      {count ? children : <div className="item-list-empty faint">{t("list.empty")}</div>}
    </div>
  );
}

/** One game: tick, picture, name and the line under it, a status, an action. A picture that fails to load
 *  leaves a plain slot, so rows keep lining up. */
export function ItemRow({ checked, onCheck, image, wide, name, sub, status, action, disabled }: {
  checked: boolean;
  onCheck: (v: boolean) => void;
  image: string;
  /** A wide store capsule instead of a square icon. */
  wide?: boolean;
  name: string;
  sub: string;
  status?: ReactNode;
  action: ReactNode;
  disabled?: boolean;
}) {
  const [broken, setBroken] = useState(false);
  return (
    <div className={`item-row${checked ? " checked" : ""}`}>
      <Check checked={checked} onChange={onCheck} disabled={disabled} title={name} />
      <span className={`item-pic${wide ? " wide" : ""}`}>
        {image && !broken && <img src={image} alt="" loading="lazy" onError={() => setBroken(true)} />}
      </span>
      <div className="item-id">
        <div className="item-name" title={name}>{name}</div>
        <div className="item-sub" title={sub}>{sub}</div>
      </div>
      <span className="item-status">{status}</span>
      <span className="item-action">{action}</span>
    </div>
  );
}
