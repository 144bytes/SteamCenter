import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import { App } from "./App";
import { TrayMenu } from "./pages/TrayMenu";
import { StoreProvider } from "./state/store";
import "./styles/index.css";

const container = document.getElementById("root");
if (!container) throw new Error("#root missing");

const tray = window.location.hash === "#tray-menu";

createRoot(container).render(
  <StrictMode>
    <StoreProvider withLogs={!tray}>
      {tray ? <TrayMenu /> : <App />}
    </StoreProvider>
  </StrictMode>,
);
