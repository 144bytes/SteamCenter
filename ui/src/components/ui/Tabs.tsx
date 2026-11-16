/** Tabs down the side of a page, apart from the main menu. */
export function Tabs<K extends string>({ items, value, onChange }: {
  items: { key: K; label: string }[];
  value: K;
  onChange: (key: K) => void;
}) {
  return (
    <div className="tabs" role="tablist">
      {items.map((item) => (
        <button key={item.key} type="button" role="tab"
                aria-selected={value === item.key}
                className={`tab${value === item.key ? " active" : ""}`}
                onClick={() => onChange(item.key)}>
          {item.label}
        </button>
      ))}
    </div>
  );
}
