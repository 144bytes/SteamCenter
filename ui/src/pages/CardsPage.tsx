import { useEffect, useRef, useState } from "react";
import { useStore } from "../state/store";
import { ipc } from "../api/ipc";
import { ActionButton, Meta, MetaRow, PageHead, SectionHead, SettingRow, SettingsGroup, Status, Switch } from "../components/ui";
import { SignedIn } from "../components/SignIn";
import { useSaveSettings } from "../hooks/useSaveSettings";
import type { CardGame, Idle } from "../api/types";

export function CardsPage() {
  const { t } = useStore();
  return (
    <>
      <PageHead title={t("cards.title")} sub={t("cards.sub")} />
      <SignedIn><Cards /></SignedIn>
    </>
  );
}

/** Seconds left of the phase, counted down on screen. */
function useLeft(idle: Idle): number {
  const [now, setNow] = useState(() => Date.now() / 1000);
  useEffect(() => {
    if (!idle.running) return;
    const id = setInterval(() => setNow(Date.now() / 1000), 1000);
    return () => clearInterval(id);
  }, [idle.running]);
  return idle.phase_ends ? Math.max(0, Math.round(idle.phase_ends - now)) : 0;
}

function Cards() {
  const { snapshot, t, act } = useStore();
  const save = useSaveSettings();
  const idle = snapshot!.idle;
  const skipVac = snapshot!.settings.skip_vac;
  const left = useLeft(idle);
  const [loading, setLoading] = useState(false);

  const reload = async () => {
    setLoading(true);
    await act(() => ipc.loadCards());
    setLoading(false);
  };
  // The list is read once per run of the program; the idling reads it again by itself each cycle.
  const asked = useRef(false);
  useEffect(() => {
    if (!idle.loaded && !idle.running && !asked.current) {
      asked.current = true;
      void reload();
    }
  });

  const skipped = (g: CardGame) => skipVac && g.vac;
  const playable = idle.games.filter((g) => !skipped(g));
  const time = `${Math.floor(left / 60)}:${String(left % 60).padStart(2, "0")}`;
  const solo = idle.phase === "solo" ? idle.games.find((g) => g.appid === idle.active[0])?.name ?? "" : "";

  return (
    <>
      <div className="card">
        <MetaRow>
          <Meta label={t("cards.games")} value={String(playable.length)} />
          <Meta label={t("cards.left")} value={String(playable.reduce((n, g) => n + g.cards, 0))} />
          <Meta label={t("cards.vac_skipped")} value={String(idle.games.filter(skipped).length)} />
          <Meta label={t("cards.cycle")} value={idle.cycle ? String(idle.cycle) : "-"} />
        </MetaRow>
      </div>

      <div className="card idle-bar">
        <span className="idle-state">
          {idle.running
            ? <Status state={idle.phase === "loading" || idle.phase === "pause" ? "connecting" : "connected"}
                      label={t.dyn(`cards.phase.${idle.phase}`, { time, name: solo, n: idle.active.length })} />
            : <Status state="idle" label={t("cards.stopped")} />}
        </span>
        {idle.running
          ? <ActionButton onClick={() => void act(() => ipc.idleStop())}>{t("cards.stop")}</ActionButton>
          : <ActionButton variant="primary" disabled={loading || !playable.length}
                          onClick={() => void act(() => ipc.idleStart(), t("cards.started"))}>
              {t("cards.start")}
            </ActionButton>}
      </div>

      <div className="card">
        <SettingsGroup>
          <SettingRow label={t("cards.skip_vac")} hint={t("cards.skip_vac_hint")}>
            <Switch checked={skipVac} onChange={(v) => save((s) => { s.skip_vac = v; })} />
          </SettingRow>
        </SettingsGroup>
      </div>

      <SectionHead title={t("cards.list")} action={
        <ActionButton disabled={loading || idle.running} onClick={() => void reload()}>
          {loading ? t("common.loading") : t("common.refresh")}
        </ActionButton>
      } />
      <div className="card">
        {loading && !idle.loaded
          ? <div className="muted icon-line"><span className="spinner" />{t("cards.reading")}</div>
          : idle.games.length ? idle.games.map((g) => (
            <div key={g.appid} className="card-row">
              <span className="item-pic wide">
                <img src={g.image} alt="" loading="lazy" onError={(e) => { e.currentTarget.style.visibility = "hidden"; }} />
              </span>
              <div className="item-id">
                <div className="item-name" title={g.name}>{g.name}</div>
                <div className="item-sub">{t("cards.game_line", { n: g.cards, hours: t.hours(g.hours * 60) })}</div>
              </div>
              <span className="item-status">
                {idle.active.includes(g.appid) ? <Status state="connected" label={t("cards.running")} />
                  : skipped(g) ? <Status state="error" label={t("cards.vac")} />
                  : <Status state="idle" label={g.vac ? t("cards.waiting_vac") : t("cards.waiting")} />}
              </span>
            </div>
          ))
          : <p className="faint backup-empty">{idle.loaded ? t("cards.none") : t("cards.not_read")}</p>}
      </div>
    </>
  );
}
