import { useStore } from "../state/store";
import { ActionButton, PageHead } from "../components/ui";
import { LogPanel } from "../components/LogPanel";
import { useSaveReport } from "../hooks/useSaveReport";

export function DiagnosticsPage() {
  const { t } = useStore();
  const saveReport = useSaveReport();
  return (
    <>
      <PageHead title={t("diag.title")} sub={t("diag.sub")} actions={
        <ActionButton onClick={() => void saveReport()}>{t("diag.report")}</ActionButton>
      } />
      <LogPanel />
    </>
  );
}
