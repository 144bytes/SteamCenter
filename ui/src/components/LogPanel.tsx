import { useState } from "react";
import { useLogs, useStore } from "../state/store";
import { ipc } from "../api/ipc";
import { ActionButton, SectionHead, Select, Switch } from "./ui";
import { useAutoScroll } from "../hooks/useAutoScroll";

const LEVELS = ["ALL", "INFO", "WARNING", "ERROR"] as const;

/** The app's log: level filter, autoscroll, the folder with the log files. */
export function LogPanel() {
  const { t, act } = useStore();
  const { logs, clearLogs } = useLogs();
  const [level, setLevel] = useState<(typeof LEVELS)[number]>("ALL");
  const [autoscroll, setAutoscroll] = useState(true);

  // A level shows its own lines; "WARNING" shows errors too.
  const visible = logs.filter((l) => level === "ALL" || l.level === level || (level === "WARNING" && l.level === "ERROR"));
  // The last line, not the count: once the log is full the count stays the same.
  const ref = useAutoScroll<HTMLDivElement>(visible[visible.length - 1], autoscroll);

  return (
    <>
      <SectionHead title={t("diag.logs")} action={
        <ActionButton onClick={() => void act(() => ipc.openLogs())}>{t("diag.open_logs")}</ActionButton>
      } />
      <div className="filters">
        <Select value={level} title={t("diag.level")} onChange={setLevel}
                options={LEVELS.map((l) => ({ value: l, label: `${t("diag.level")}: ${t(`diag.level.${l}`)}` }))} />
        <Switch checked={autoscroll} onChange={setAutoscroll} label={t("diag.autoscroll")} />
        <span className="spacer" />
        <ActionButton onClick={clearLogs}>{t("diag.clear")}</ActionButton>
      </div>
      <div className="log-view" ref={ref}>
        {visible.map((line) => (
          <div className={`log-line ${line.level}`} key={line.id}>
            <span className="log-ts">{line.ts} </span>
            {line.module && <span className="log-mod">[{line.module}] </span>}
            {line.message}
          </div>
        ))}
        {!visible.length && <div className="faint">-</div>}
      </div>
    </>
  );
}
