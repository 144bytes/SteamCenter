import { useCallback } from "react";
import { errorMsg, ipc } from "../api/ipc";
import { useStore } from "../state/store";

/** "Save report": asks where to save the support report and tells where it went. */
export function useSaveReport() {
  const { t, notify } = useStore();
  return useCallback(async () => {
    try {
      const path = await ipc.saveReport(t("diag.report_filter"));
      if (path) notify(t("diag.report_saved", { path }), "ok");
    } catch (err) {
      notify(t.msg(errorMsg(err)), "err");
    }
  }, [t, notify]);
}
