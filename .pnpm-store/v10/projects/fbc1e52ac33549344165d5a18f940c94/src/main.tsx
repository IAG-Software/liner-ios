import React from "react";
import ReactDOM from "react-dom/client";
import { installLinerBridge } from "@/tauri/bridge";
import { canPreview, installPreviewBackend, isPreviewRequested } from "@/tauri/preview";
import { initGlobalErrorHandlers } from "@/shared/utils/globalErrorHandlers";
import { hydrateScreenCache } from "@/shared/cache/screenCache";
import { applyStoredTheme } from "@/shared/utils/storedTheme";

initGlobalErrorHandlers();

// `?preview=1` in a plain browser: fake the native backend so screens past the
// login wall can be reviewed at phone dimensions. No-op inside the app.
if (import.meta.env.DEV && isPreviewRequested() && canPreview()) {
  installPreviewBackend();
}

const isOverlay = window.location.hash.startsWith("#/overlay");

if (!isOverlay) applyStoredTheme();

/** Mirror a line into Documents/liner-boot.log (no console on iOS). */
function bootLog(message: string): void {
  try {
    const internals = (
      window as unknown as {
        __TAURI_INTERNALS__?: {
          invoke: (cmd: string, args?: unknown) => Promise<unknown>;
        };
      }
    ).__TAURI_INTERNALS__;
    void internals?.invoke("boot_log", { message: `[web] ${message}` });
  } catch {
    /* diagnostics must never break boot */
  }
}

window.addEventListener("DOMContentLoaded", () => bootLog("DOMContentLoaded"));
bootLog("main.tsx evaluating");

async function loadRoot(): Promise<React.ReactElement> {
  if (isOverlay) {
    const { default: OverlayApp } = await import("./app/OverlayApp");
    return <OverlayApp />;
  }
  bootLog("importing app modules");
  const [
    { initMainWindowServices },
    { default: App },
    { default: RootErrorBoundary },
  ] = await Promise.all([
    import("./app/mainWindowServices"),
    import("./app/App"),
    import("@/shared/ui/RootErrorBoundary"),
    hydrateScreenCache(),
  ]);
  bootLog("app modules imported");
  initMainWindowServices();
  return (
    <RootErrorBoundary>
      <App />
    </RootErrorBoundary>
  );
}

const rootElement = document.getElementById("root");

if (rootElement) {
  // The native bridge must exist before the first render: several stores read
  // `window.linerElectron.authSession.get()` (a synchronous call, mirroring the
  // old `sendSync` IPC) while their initial state is computed.
  void installLinerBridge()
    .then(() => bootLog("bridge installed"))
    .catch((error) => {
      bootLog(`bridge failed: ${String(error)}`);
      console.error("[liner] native bridge failed to install", error);
    })
    .then(() => loadRoot())
    .then((root) => {
      ReactDOM.createRoot(rootElement).render(
        <React.StrictMode>{root}</React.StrictMode>,
      );
      // Give React two frames to commit, then tell the boot guard it worked.
      requestAnimationFrame(() =>
        requestAnimationFrame(() => {
          (
            window as unknown as { __LINER_MOUNTED?: boolean }
          ).__LINER_MOUNTED = true;
          bootLog("react mounted");
        }),
      );
    })
    .catch((error) => {
      bootLog(`boot failed: ${String(error?.stack ?? error)}`);
      throw error;
    });
}
