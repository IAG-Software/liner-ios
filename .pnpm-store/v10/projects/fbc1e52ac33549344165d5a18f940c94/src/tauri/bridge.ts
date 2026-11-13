/**
 * `window.linerElectron` — Tauri-backed implementation.
 *
 * Why this shape: the React renderer (7680 modules) was written against an
 * Electron contextBridge object. Rewriting every call site to `invoke()`
 * would have meant touching ~135 locations across auth, caching, signing,
 * libraries, the player and eight feature stores — each one a chance to
 * regress. Instead the same object is re-created here, backed by Tauri
 * commands. The renderer is byte-identical to the desktop build.
 *
 * The only behavioural difference the renderer can observe is
 * `platform !== "darwin" | "linux"`, which is what puts it into the touch
 * layout.
 */

import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";

import { installHttpProxy } from "./httpProxy";
import type {
  AuthRefreshResult,
  DeeplinkTarget,
  DiscordStatus,
  LinerCacheStats,
  LinerNativeApi,
  LocalImportResult,
  LocalLibraryUsage,
  LocalTrackRecord,
  SavedLibraryUsage,
  SavedLyricsState,
  SavedTrackInput,
  SavedTrackRecord,
  SavedTracksPartUsage,
  SignInput,
  SignResult,
  StoredAuthSession,
  WindowState,
} from "./api";

/* ------------------------------------------------------------------ */
/* helpers                                                             */
/* ------------------------------------------------------------------ */

const noop = () => {};
const unsub = () => noop;

/** Subscribe to a Tauri event, returning a synchronous unsubscribe. */
function subscribe<T>(event: string, cb: (payload: T) => void): () => void {
  let disposed = false;
  let unlisten: UnlistenFn | null = null;

  void listen<T>(event, (message) => {
    if (!disposed) cb(message.payload);
  }).then((off) => {
    if (disposed) off();
    else unlisten = off;
  });

  return () => {
    disposed = true;
    unlisten?.();
    unlisten = null;
  };
}

/**
 * PostMessage cannot carry a `Uint8Array`, and a JSON array of 5 M numbers
 * costs ~20 MB of string. Base64 is ~33 % overhead and decodes in the Rust
 * command.
 */
function toBase64(bytes: Uint8Array): string {
  let binary = "";
  const chunk = 0x8000;
  for (let i = 0; i < bytes.length; i += chunk) {
    binary += String.fromCharCode(
      ...bytes.subarray(i, Math.min(i + chunk, bytes.length)),
    );
  }
  return btoa(binary);
}

function detectPlatform(): string {
  const ua = navigator.userAgent;
  // iPadOS reports itself as "Macintosh" but is touch-capable.
  if (/iPhone|iPad|iPod/.test(ua)) return "ios";
  if (/Macintosh/.test(ua) && navigator.maxTouchPoints > 1) return "ios";
  return "ios";
}

/* ------------------------------------------------------------------ */
/* boot                                                                */
/* ------------------------------------------------------------------ */

let linerSession: StoredAuthSession | null = null;
let youtubeSession: StoredAuthSession | null = null;

/**
 * Install the bridge and warm the two synchronous caches.
 *
 * `authSession.get()` has to be synchronous — the renderer calls it during the
 * first render to decide whether to show the sign-in screen. Electron solved
 * this with `ipcRenderer.sendSync`; Tauri has no sync IPC, so the value is
 * fetched once here and kept in memory, and every later mutation is pushed by
 * the `auth:session-changed` event.
 */
export async function installLinerBridge(): Promise<void> {
  const platform = detectPlatform();

  // Before anything else can issue a request: Liner hosts must leave through
  // the native client (WAF User-Agent + no CORS), everything else stays on
  // WebKit.
  installHttpProxy();

  const [cachedLiner, cachedYoutube, version] = await Promise.allSettled([
    invoke<StoredAuthSession | null>("auth_get_session"),
    invoke<StoredAuthSession | null>("youtube_auth_get_session"),
    invoke<string>("app_get_version"),
  ]);

  if (cachedLiner.status === "fulfilled") linerSession = cachedLiner.value;
  if (cachedYoutube.status === "fulfilled") youtubeSession = cachedYoutube.value;

  const appVersion = version.status === "fulfilled" ? version.value : "0.0.0";

  subscribe<StoredAuthSession | null>("auth:session-changed", (session) => {
    linerSession = session;
  });

  const api: LinerNativeApi = {
    platform,
    isHyprland: false,

    /* ---- window chrome: a single OS-managed window on iOS ---- */
    minimize: noop as () => Promise<void>,
    toggleMaximize: noop as () => Promise<void>,
    toggleFullScreen: noop as () => Promise<void>,
    close: noop as () => Promise<void>,
    isMaximized: async () => false,
    isFullScreen: async () => false,
    getRememberWindowBounds: async () => true,
    setRememberWindowBounds: async () => true,
    onWindowStateChange: (_cb: (state: WindowState) => void) => unsub(),
    onWindowRestored: (_cb: () => void) => unsub(),
    onWindowMinimized: (_cb: () => void) => unsub(),
    notePresence: noop,
    dragStart: noop,
    dragMove: noop as (x: number, y: number) => void,
    dragEnd: noop,

    /* ---- signing (native HMAC, no WASM) ---- */
    signRequest: (input: SignInput) => invoke<SignResult>("signer_sign_request", { input }),
    signMonitorRequest: (input: SignInput) =>
      invoke<SignResult>("signer_sign_monitor_request", { input }),
    syncServerTime: (dateHeaderOrMs: string | number) =>
      invoke<number>("signer_sync_time", { dateHeaderOrMs }),
    setEdgeKey: (key: string | null) => {
      void invoke("edge_set_key", { key });
    },
    getServerTimeOffset: () => invoke<number>("signer_get_time_offset"),
    signCoverUrl: (payload: string) => invoke<string>("signer_sign_cover_url", { payload }),
    signRawPayload: (payload: number[] | Uint8Array) =>
      invoke<string>("signer_sign_raw_payload", {
        payload: Array.from(payload),
      }),

    /* ---- auth ---- */
    authSession: {
      get: () => linerSession,
      set: async (next) => {
        const value = next
          ? { accessToken: next.accessToken, refreshToken: next.refreshToken }
          : null;
        const stored = await invoke<StoredAuthSession | null>("auth_set_session", { next: value });
        linerSession = stored;
        return stored;
      },
      refresh: async (input) => {
        const result = await invoke<AuthRefreshResult>("auth_refresh", { input });
        linerSession = result.session;
        return result;
      },
      onChange: (cb: (session: StoredAuthSession | null) => void) =>
        subscribe<StoredAuthSession | null>("auth:session-changed", cb),
    },
    youtubeAuthSession: {
      get: () => youtubeSession,
      set: async (next) => {
        const stored = await invoke<StoredAuthSession | null>("youtube_auth_set_session", {
          next: next
            ? { accessToken: next.accessToken, refreshToken: next.refreshToken }
            : null,
        });
        youtubeSession = stored;
        return stored;
      },
    },

    /* ---- misc ---- */
    onDeeplink: (_cb: (target: DeeplinkTarget) => void) => unsub(),
    onOpenUrl: (cb: (url: string) => void) =>
      subscribe<string>("deep-link", cb),
    diagnoseNetwork: (hosts: string[]) => invoke("net_diagnose", { hosts }),
    probeNetwork: () => invoke("net_probe"),
    openDownloads: async () => false,
    openExportFolder: async () => false,
    setAppIcon: async () => {},
    getAppVersion: async () => appVersion,
    getCacheStats: () => invoke<LinerCacheStats>("storage_get_cache_stats"),
    openCacheFolder: () => invoke<boolean>("storage_open_cache_folder"),
    clearCoversCache: () => invoke<boolean>("storage_clear_covers_cache"),
    clearAudioCache: (clearEntireDb?: boolean) =>
      invoke<boolean>("storage_clear_audio_cache", { clearEntireDb }),
    clearHttpCache: () => invoke<boolean>("storage_clear_http_cache"),

    /* ---- updater: sideloaded builds update out of band ---- */
    checkForUpdates: async () => ({ available: false }),
    downloadUpdate: async () => ({ success: false, error: "unsupported on ios" }),
    quitAndInstall: async () => false,
    onUpdateAvailable: (_cb) => unsub(),
    onUpdateDownloadProgress: (_cb) => unsub(),
    onUpdateDownloaded: (_cb) => unsub(),
    onUpdateError: (_cb) => unsub(),

    /* ---- OTA bundle: the bundle is sealed inside the .ipa ---- */
    bundleCheck: async () => ({ available: false, reason: "sealed bundle" }),
    bundleDownloadAndInstall: async () => ({
      success: false,
      error: "unsupported on ios",
    }),
    bundleHotSwap: async () => false,
    bundleRollback: async () => false,
    bundleGetStatus: async () => ({
      appVersion,
      bundleVersion: appVersion,
      isOta: false,
      sha256: null,
      effectivePath: "",
    }),
    onBundleAvailable: (_cb) => unsub(),
    onBundleDownloadProgress: (_cb) => unsub(),
    onBundleReady: (_cb) => unsub(),
    onBundleStamp: (_cb) => unsub(),

    /* ---- touchbar / overlay: no such surfaces on iOS ---- */
    touchbarUpdateState: noop,
    onTouchBarAction: (_cb) => unsub(),
    sendTouchBarAction: noop,
    getTouchBarInitialState: async () => null,
    onTouchBarRequestSync: (_cb) => unsub(),
    overlayUpdateState: noop,
    overlayReady: noop,
    onOverlayAction: (_cb) => unsub(),
    sendOverlayAction: noop,
    toggleOverlay: async () => false,
    isOverlayOpen: async () => false,
    closeOverlay: noop,
    onOverlayState: (_cb) => unsub(),
    getOverlayInitialState: async () => null,
    updateOverlaySettings: noop,
    pauseOverlayShortcuts: noop,
    getShortcutStatus: async () => ({}),
    onShortcutStatus: (_cb) => unsub(),
    onOverlaySettingsChanged: (_cb) => unsub(),
    onOverlayVisibility: (_cb) => unsub(),
    setOverlayAuthenticated: noop,

    /* ---- discord RPC needs a local socket; unavailable in the sandbox ---- */
    discordUpdateState: noop,
    discordUpdateSettings: noop,
    getDiscordStatus: async (): Promise<DiscordStatus> => ({ connected: false }),

    /* ---- youtube helpers that require a Node worker ---- */
    mintPoToken: async () => {
      // BotGuard needs a dedicated worker process on desktop. In WKWebView the
      // renderer's own web path is used instead; callers already handle the
      // 501 that upstream throws when the bridge is absent.
      throw new Error("PO token minting is unavailable on iOS");
    },
    youtubeTvSession: async () => {
      throw new Error("YouTube TV playback requires the desktop app");
    },
    youtubeSolveN: async () => {
      throw new Error("YouTube TV playback requires the desktop app");
    },

    /* ---- libraries ---- */
    localTracksList: () => invoke<LocalTrackRecord[]>("local_tracks_list"),
    localTracksImport: (paths: string[]) =>
      invoke<LocalImportResult>("local_tracks_import", { paths }),
    localTracksPick: () => invoke<LocalImportResult | null>("local_tracks_pick"),
    localTracksRemove: async (ids: string[]) => {
      await invoke("local_tracks_remove", { ids });
    },
    localTracksRemoveAll: () => invoke<number>("local_tracks_remove_all"),
    localTracksUsage: () => invoke<LocalLibraryUsage>("local_tracks_usage"),

    savedTracksList: () => invoke<SavedTrackRecord[]>("saved_tracks_list"),
    savedTracksSave: async (input: SavedTrackInput) =>
      invoke<SavedTrackRecord | null>("saved_tracks_save", {
        input: {
          meta: input.meta,
          audio: toBase64(input.audio),
          mimeType: input.mimeType,
          cover: input.cover
            ? {
                data: toBase64(input.cover.data),
                mimeType: input.cover.mimeType,
              }
            : null,
          lyricsState: input.lyricsState,
          lyrics: input.lyrics,
        },
      }),
    savedTracksRemove: (trackIds: string[]) =>
      invoke<number>("saved_tracks_remove", { trackIds }),
    savedTracksRemoveAll: () => invoke<number>("saved_tracks_remove_all"),
    savedTracksUsage: () => invoke<SavedLibraryUsage>("saved_tracks_usage"),
    savedTracksUsageOf: (trackIds: string[]) =>
      invoke<SavedTracksPartUsage>("saved_tracks_usage_of", { trackIds }),
    savedTracksOpenFolder: async () => false,
    savedTracksGetLyrics: (trackId: string) =>
      invoke<unknown | null>("saved_tracks_get_lyrics", { trackId }),
    savedTracksSetLyrics: (
      trackId: string,
      state: SavedLyricsState,
      lyrics?: unknown,
    ) =>
      invoke<boolean>("saved_tracks_set_lyrics", {
        trackId,
        lyricsState: state,
        lyrics,
      }),

    /**
     * Electron exposed the real path of a dropped `File`. There is no such
     * thing in the sandbox, and the renderer only uses it for desktop
     * drag-and-drop, so returning the name keeps the type honest.
     */
    pathForFile: (file: File) => file.name,

    nowPlaying: {
      update: (state) =>
        invoke<void>("now_playing_update", {
          title: state.title,
          artist: state.artist,
          album: state.album ?? null,
          durationMs: state.durationMs,
          positionMs: state.positionMs,
          playing: state.playing,
          artworkPath: state.artworkPath ?? null,
        }),
      elapsed: (positionMs: number, rate: number) =>
        invoke<void>("now_playing_elapsed", { positionMs, rate }),
      clear: () => invoke<void>("now_playing_clear"),
    },
    showRoutePicker: () => invoke<boolean>("show_route_picker"),
    onRemoteCommand: (cb: (command: string) => void) =>
      subscribe<string>("media:remote", cb),
    onRemoteSeek: (cb: (seconds: number) => void) =>
      subscribe<number>("media:remote-seek", cb),
  };

  window.linerElectron = api;
}

/** Exposed for the native-audio adapter and diagnostics. */
export function linerBridge(): LinerNativeApi {
  if (!window.linerElectron) {
    throw new Error("Liner bridge installed after first render");
  }
  return window.linerElectron;
}
