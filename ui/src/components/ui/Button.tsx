import type { ButtonHTMLAttributes } from "react";

/** `ghost` and `ghost-danger` are text without a frame: Restart and Delete in a card. The rest are framed buttons. */
export type ButtonVariant =
  | "primary" | "secondary" | "ghost" | "ghost-danger" | "danger" | "icon";

type ButtonLook = {
  variant?: ButtonVariant;
  /** md: all buttons; sm: the × of a window.
   *  Each size has one width and one height, whatever the text says. */
  size?: "md" | "sm";
  /** For `icon`: no frame and a faint colour, like the × of a window. */
  quiet?: boolean;
};

/** The one button of the app. */
export function ActionButton({ variant = "secondary", size = "md", quiet, className, children, title,
                               ...rest }: ButtonLook & ButtonHTMLAttributes<HTMLButtonElement>) {
  // The label is also the tooltip: a fixed width can cut a long word short.
  const label = typeof children === "string" ? children : undefined;
  const cls = ["ab", variant, size === "sm" && "sm", quiet && "ghost", className].filter(Boolean).join(" ");
  return (
    <button type="button" className={cls} title={title ?? label} {...rest}>
      {label !== undefined ? <span>{label}</span> : children}
    </button>
  );
}
