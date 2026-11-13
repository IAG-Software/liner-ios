import { initTelemetry } from "@/shared/telemetry";
import {
  forgetStorageAnalytics,
  initBackgroundStorageAnalytics,
} from "@/shared/utils/cacheManager";
import { api, initEdgeKey, registerSignOutTask } from "@/shared/api";
import { initClientVisitorSession } from "@/shared/youtube";
import { initBundleStampLog } from "@/shared/utils/bundleStampLog";
import { initSavedTracks } from "@/features/library/saved";
import { initOfflineMode } from "@/features/connectivity/engine/offlineController";
import { installNativeMediaBridge } from "@/features/player/engine/nativeMediaBridge";

export function initMainWindowServices() {
  initTelemetry();
  initBackgroundStorageAnalytics();
  initClientVisitorSession();
  initBundleStampLog();
  initOfflineMode();
  initSavedTracks();
  initEdgeKey(() => api.fetchEdgeKey());
  registerSignOutTask(forgetStorageAnalytics);
  // Lock screen / Dynamic Island / Siri transport, forwarded from the native
  // Now Playing centre into the web player.
  installNativeMediaBridge();
}
