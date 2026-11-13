/**
 * Desktop preview harness — `?preview=1`.
 *
 * Runs the *real* renderer against a fake Tauri backend so screens past the
 * login wall can be reviewed at phone dimensions in a normal browser:
 *
 *     pnpm dev
 *     http://localhost:5173/?preview=1
 *
 * It answers the commands the shell needs (session, libraries, version) and
 * stubs `fetch` for Liner hosts, which is enough to render navigation, the tab
 * bar, the mini player and settings without a device or a login.
 *
 * Never compiled into a production iOS build: it installs only when the page
 * has no `__TAURI_INTERNALS__` and the query string asks for it.
 */

const PREVIEW_SESSION = {
  accessToken: "preview-access-token",
  refreshToken: "preview-refresh-token",
  user: {
    id: "preview-user",
    username: "preview",
    displayName: "Preview User",
    avatarUrl: "",
  },
};

const noop = () => Promise.resolve(null);

export function isPreviewRequested(): boolean {
  if (typeof window === "undefined") return false;
  return new URLSearchParams(window.location.search).has("preview");
}

export function canPreview(): boolean {
  if (typeof window === "undefined") return false;
  const internals = (window as unknown as { __TAURI_INTERNALS__?: unknown })
    .__TAURI_INTERNALS__;
  return !internals;
}

export function installPreviewBackend(): void {
  let callbackId = 1;

  const handlers: Record<string, (args: any) => unknown> = {
    app_get_version: () => "0.2.14-preview",
    app_platform: () => "ios",
    app_set_icon: noop,
    app_open_external: () => true,

    auth_get_session: () => PREVIEW_SESSION,
    auth_set_session: (args) => args?.next ?? null,
    auth_refresh: () => ({
      outcome: "refreshed",
      session: PREVIEW_SESSION,
      reason: null,
    }),
    youtube_auth_get_session: () => null,
    youtube_auth_set_session: () => null,

    signer_sign_request: () => ({ signature: "preview", timestamp: 0, nonce: "0" }),
    signer_sign_monitor_request: () => ({
      signature: "preview",
      timestamp: 0,
      nonce: "0",
    }),
    signer_sync_time: () => 0,
    signer_get_time_offset: () => 0,
    signer_sign_cover_url: (args) => args?.payload ?? "",
    signer_sign_raw_payload: () => "preview",
    edge_set_key: noop,

    storage_get_cache_stats: () => ({
      cacheSize: 0,
      httpCacheSize: 0,
      codeCacheSize: 0,
      serviceWorkerSize: 0,
      indexedDbSize: 0,
      localStorageSize: 0,
      blobStorageSize: 0,
      cachePath: "/preview/Caches",
    }),
    storage_clear_covers_cache: () => true,
    storage_clear_audio_cache: () => true,
    storage_clear_http_cache: () => true,
    storage_open_cache_folder: () => false,
    shell_open_downloads: () => false,
    shell_open_export_folder: () => false,

    local_tracks_list: () => [],
    local_tracks_import: () => ({ added: [], duplicates: 0, failed: [] }),
    local_tracks_pick: () => null,
    local_tracks_remove: noop,
    local_tracks_remove_all: () => 0,
    local_tracks_usage: () => ({ count: 0, bytes: 0 }),

    saved_tracks_list: () => [],
    saved_tracks_save: () => null,
    saved_tracks_remove: () => 0,
    saved_tracks_remove_all: () => 0,
    saved_tracks_usage: () => ({ count: 0, bytes: 0, freeBytes: null }),
    saved_tracks_usage_of: () => ({ count: 0, bytes: 0 }),
    saved_tracks_open_folder: () => false,
    saved_tracks_get_lyrics: () => null,
    saved_tracks_set_lyrics: () => false,

    net_diagnose: () => ({
      dns: [],
      tcp: [],
      tls: [],
      controls: [],
      trace: null,
      platform: "ios",
    }),
    net_probe: () => ({
      internet: true,
      youtube: true,
      youtubeFailure: null,
      country: "SA",
    }),

    audio_state: () => ({
      trackId: null,
      positionMs: 0,
      durationMs: 0,
      playing: false,
      buffering: false,
      volume: 1,
      rate: 1,
      queueLen: 0,
      queueIndex: 0,
    }),
    lyrics_load: () => 0,
    lyrics_clear: noop,
    lyrics_current: () => ({
      positionMs: 0,
      frame: 0,
      lineIndex: null,
      wordIndex: null,
      lineProgress: 0,
    }),

    boot_log: noop,
    boot_log_path: () => "/preview/liner-boot.log",
  };

  const internals = {
    // `listen()` goes through the event plugin; a numeric id is all it needs.
    transformCallback(callback: (payload: unknown) => void) {
      const id = callbackId++;
      (window as unknown as Record<string, unknown>)[`_${id}`] = callback;
      return id;
    },
    invoke(command: string, args?: any) {
      if (command === "plugin:event|listen") return Promise.resolve(1);
      if (command === "plugin:event|unlisten") return Promise.resolve();
      const handler = handlers[command];
      if (!handler) {
        // Unknown command: resolve empty instead of throwing so a missing mock
        // degrades a screen rather than blanking the app.
        return Promise.resolve(null);
      }
      try {
        return Promise.resolve(handler(args ?? {}));
      } catch (error) {
        return Promise.reject(error);
      }
    },
    metadata: { currentWindow: { label: "main" }, currentWebview: { label: "main" } },
  };

  (window as unknown as Record<string, unknown>).__TAURI_INTERNALS__ = internals;

  // Network: answer Liner hosts with empty JSON so pages land in their
  // empty/error states instead of hanging on a dead API.
  const realFetch = window.fetch.bind(window);
  window.fetch = async (input: RequestInfo | URL, init?: RequestInit) => {
    const url =
      typeof input === "string"
        ? input
        : input instanceof URL
          ? input.toString()
          : input.url;

    let host = "";
    try {
      host = new URL(url, window.location.href).hostname;
    } catch {
      return realFetch(input as RequestInfo, init);
    }

    if (!/(^|\.)tryliner\.fun$/i.test(host)) {
      return realFetch(input as RequestInfo, init);
    }

    if (url.includes("/v1/me")) {
      return new Response(JSON.stringify(PREVIEW_SESSION.user), {
        status: 200,
        headers: { "content-type": "application/json" },
      });
    }

    // Generic empty payload: arrays for collections, {} otherwise.
    const body = /(list|search|feed|home|library|tracks|albums|artists|playlists)/i.test(url)
      ? "[]"
      : "{}";
    return new Response(body, {
      status: 200,
      headers: { "content-type": "application/json" },
    });
  };
}
