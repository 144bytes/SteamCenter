import { useStore } from "../../state/store";
import type { State } from "../../api/types";

/** Dot colour for a state. */
const TONE: Record<State, "ok" | "err" | "idle" | "busy"> = {
  connected: "ok", connecting: "busy", error: "err", idle: "idle",
};

/** The only way a status is drawn: one dot and one word. `label` replaces the state's own word. */
export function Status({ state, label }: { state: State; label?: string }) {
  const { t } = useStore();
  const text = label ?? t(`state.${state}`);
  return (
    <span className="status">
      <span className={`dot ${TONE[state]}`} />
      {text && <span className="status-text" title={text}>{text}</span>}
    </span>
  );
}
