import { useEffect, useRef } from "react";
import type { ReactNode } from "react";
import { ActionButton } from "./Button";
import { useStore } from "../../state/store";
import type { ButtonVariant } from "./Button";

// The windows open right now, newest last: Esc closes only the top one.
const openWindows: object[] = [];

/** A window. It never closes on a click beside it - a stray click would
 *  lose what was typed - only on Cancel, × and Esc. */
export function Modal({ title, onClose, children, footer }: {
  title: string;
  onClose: () => void;
  children: ReactNode;
  footer?: ReactNode;
}) {
  const { t } = useStore();
  const close = useRef(onClose);
  close.current = onClose;

  useEffect(() => {
    const me = {};
    openWindows.push(me);
    const onKey = (e: KeyboardEvent) => {
      if (e.key !== "Escape" || openWindows[openWindows.length - 1] !== me) return;
      e.stopPropagation();
      close.current();
    };
    window.addEventListener("keydown", onKey);
    return () => {
      window.removeEventListener("keydown", onKey);
      openWindows.splice(openWindows.indexOf(me), 1);
    };
  }, []);

  return (
    <div className="overlay">
      <div className="modal" role="dialog" aria-modal="true">
        <div className="modal-head">
          <span className="modal-title" title={title}>{title}</span>
          <ActionButton variant="icon" size="sm" quiet onClick={onClose}
                        aria-label={t("common.close")} title={t("common.close")}>
            ✕
          </ActionButton>
        </div>
        <div className="modal-body">{children}</div>
        {footer && <div className="modal-foot">{footer}</div>}
      </div>
    </div>
  );
}

/** Cancel and Save, the footer of every form; a confirmation names its own action. */
export function FormFooter({ onCancel, onConfirm, busy, label, variant = "primary" }: {
  onCancel: () => void;
  onConfirm: () => void;
  busy?: boolean;
  label?: string;
  variant?: ButtonVariant;
}) {
  const { t } = useStore();
  return (
    <>
      <ActionButton onClick={onCancel}>{t("common.cancel")}</ActionButton>
      <ActionButton variant={variant} onClick={onConfirm} disabled={busy}>
        {label ?? t("common.save")}
      </ActionButton>
    </>
  );
}

/** A question before an action: it names the action on its button; `danger` - one that takes something away. */
export function Confirm({ text, title, action, danger, onCancel, onConfirm }: {
  text: string; title: string; action: string; danger?: boolean;
  onCancel: () => void; onConfirm: () => void;
}) {
  return (
    <Modal title={title} onClose={onCancel} footer={
      <FormFooter onCancel={onCancel} onConfirm={onConfirm} label={action} variant={danger ? "danger" : "primary"} />
    }>
      <p className="field">{text}</p>
    </Modal>
  );
}
