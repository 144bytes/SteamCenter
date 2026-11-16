import { useEffect, useRef } from "react";
import type { InputHTMLAttributes, ReactNode } from "react";

/** A caption, a control and a hint under it. */
export function Field({ label, hint, children }: {
  label?: string; hint?: ReactNode; children: ReactNode;
}) {
  return (
    <label className="field">
      {label && <span className="field-label">{label}</span>}
      {children}
      {hint && <span className="field-hint">{hint}</span>}
    </label>
  );
}

export function TextInput({ value, onChange, className, ...rest }: {
  value: string;
  onChange: (value: string) => void;
} & Omit<InputHTMLAttributes<HTMLInputElement>, "value" | "onChange">) {
  return <input className={["input", className].filter(Boolean).join(" ")} value={value}
                onChange={(e) => onChange(e.target.value)} {...rest} />;
}

/** A picker: the one fixed width of every select. */
export function Select<V extends string>({ value, onChange, options, disabled, title }: {
  value: V;
  onChange: (value: V) => void;
  options: { value: V; label: string }[];
  disabled?: boolean;
  title?: string;
}) {
  return (
    <select className="select pick" value={value}
            disabled={disabled} title={title}
            onChange={(e) => onChange(e.target.value as V)}>
      {options.map((o) => <option key={o.value} value={o.value}>{o.label}</option>)}
    </select>
  );
}

export function Switch({ checked, onChange, label }: {
  checked: boolean;
  onChange: (v: boolean) => void;
  label?: string;
}) {
  return (
    <label className="switch">
      <input type="checkbox" checked={checked}
             onChange={(e) => onChange(e.target.checked)} />
      <span className="switch-track"><span className="switch-thumb" /></span>
      {label && <span>{label}</span>}
    </label>
  );
}

/** A tick in a list row or its head. `mixed` - some of the rows are ticked. */
export function Check({ checked, mixed, onChange, disabled, title }: {
  checked: boolean;
  mixed?: boolean;
  onChange: (v: boolean) => void;
  disabled?: boolean;
  title?: string;
}) {
  const ref = useRef<HTMLInputElement>(null);
  useEffect(() => {
    if (ref.current) ref.current.indeterminate = !!mixed;
  }, [mixed]);
  return (
    <input ref={ref} type="checkbox" className="check" checked={checked} disabled={disabled} title={title}
           aria-label={title} onChange={(e) => onChange(e.target.checked)} />
  );
}

/** A refusal that has just appeared in a window is kept in sight: the window's
 *  body scrolls, and it could otherwise turn up outside the visible part. */
export function FormError({ message }: { message: string | null | undefined }) {
  const ref = useRef<HTMLDivElement>(null);
  useEffect(() => {
    const el = ref.current;
    if (message) el?.scrollIntoView({ block: "nearest" });
  }, [message]);
  return message ? <div className="form-error" ref={ref}>{message}</div> : null;
}
