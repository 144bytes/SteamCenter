import { useState } from "react";
import type { ReactNode } from "react";
import { ipc } from "../api/ipc";
import { useStore } from "../state/store";
import { ActionButton } from "./ui";
import { TokenModal } from "./modals";

/** "Sign in with Steam" and "Paste a token": the two ways in, wherever the app asks for a sign-in. */
export function SignInButtons() {
  const { snapshot, t, act } = useStore();
  const [token, setToken] = useState(false);
  const waiting = snapshot?.signing_in ?? false;
  return (
    <>
      <ActionButton variant="primary" className="wide" disabled={waiting} onClick={() => void act(() => ipc.signIn())}>
        {waiting ? t("account.waiting") : t("account.sign_in")}
      </ActionButton>
      <ActionButton className="wide" onClick={() => setToken(true)}>{t("account.paste_token")}</ActionButton>
      {token && <TokenModal onClose={() => setToken(false)} />}
    </>
  );
}

/** Steam data that could not be loaded: why, "Try again" - and the way in when Steam no longer takes the sign-in. */
export function LoadError({ error, code, onRetry }: { error: string; code: string | null; onRetry: () => void }) {
  const { t } = useStore();
  return (
    <div className="card sign-in">
      <p className="form-error">{error}</p>
      <div className="sign-in-actions">
        <ActionButton onClick={onRetry}>{t("common.retry")}</ActionButton>
        {code === "err.session_expired" && <SignInButtons />}
      </div>
    </div>
  );
}

/** A page about the account shows its content only while the account is signed in; otherwise - the way to sign in. */
export function SignedIn({ children }: { children: ReactNode }) {
  const { snapshot, t } = useStore();
  const account = snapshot?.account;
  if (account?.expires) return <>{children}</>;
  return (
    <div className="card sign-in">
      <p className="section-title">{account ? t("account.expired_title") : t("account.sign_in_title")}</p>
      <p className="muted">{account ? t("account.expired_text", { name: account.name }) : t("account.sign_in_text")}</p>
      <div className="sign-in-actions"><SignInButtons /></div>
    </div>
  );
}
