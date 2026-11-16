import { useState } from "react";
import { ipc } from "../../api/ipc";
import { Field, FormError, FormFooter, Modal, TextInput } from "../ui";
import { useSubmit } from "../../hooks/useSubmit";
import { useStore } from "../../state/store";

const TOKEN_PAGE = "https://store.steampowered.com/pointssummary/ajaxgetasyncconfig";

/** Signing in with an access token copied from a browser where Steam is open: for when the sign-in window can't be
 *  used. The token is checked with Steam before it is kept. */
export function TokenModal({ onClose }: { onClose: () => void }) {
  const { t, notify } = useStore();
  const [token, setToken] = useState("");
  const { busy, error, run } = useSubmit();

  const save = async () => {
    if (await run(() => ipc.signInToken(token))) {
      notify(t("account.signed_in"), "ok");
      onClose();
    }
  };

  return (
    <Modal title={t("account.token_title")} onClose={onClose}
           footer={<FormFooter onCancel={onClose} onConfirm={() => void save()} busy={busy || !token.trim()}
                               label={t("account.sign_in_short")} />}>
      <p className="field muted">
        {t("account.token_text")}{" "}
        <button type="button" className="link" onClick={() => void ipc.openUrl(TOKEN_PAGE)}>{t("account.token_page")}</button>
      </p>
      <Field label={t("account.token")} hint={t("account.token_hint")}>
        <TextInput type="password" value={token} onChange={setToken} autoFocus spellCheck={false} autoComplete="off"
                   onKeyDown={(e) => {
                     if (e.key !== "Enter") return;
                     e.preventDefault();
                     if (!busy && token.trim()) void save();
                   }} />
      </Field>
      <FormError message={error} />
    </Modal>
  );
}
