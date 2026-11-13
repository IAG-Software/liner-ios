/**
 * The bridge contract.
 *
 * This is a 1:1 restatement of `LinerElectronApi` from the desktop preload
 * (`electron/preload.ts`). The renderer was written against that object and is
 * unchanged, so the port keeps the exact same shape and swaps the transport:
 * every member below is backed by a `#[tauri::command]` in `src-tauri/src`.
 *
 * Members that only ever made sense on a desktop (custom window chrome, the
 * Touch Bar, the Discord RPC socket, the OTA frontend bundle, the overlay
 * window) are implemented as inert stubs: they resolve with the "nothing
 * happened" value so feature-detection code in the renderer falls through
 * instead of throwing.
 */

export interface DiscordStatus {
  connected: boolean;
  username?: string;
  avatarUrl?: string;
  viaArrpc?: boolean;
}

export interface DeeplinkTarget {
  type: "artist" | "album" | "playlist" | "track";
  id: string;
}

export interface StoredAuthSession {
  accessToken: string;
  refreshToken: string;
  user?: unknown;
  [key: string]: unknown;
}

export type AuthRefreshOutcome =
  | "refreshed"
  | "rotated"
  | "signed_out"
  | "transient"
  | "no_session";

export interface AuthRefreshResult {
  outcome: AuthRefreshOutcome;
  session: StoredAuthSession | null;
  reason?: string;
}

export interface LinerCacheStats {
  cacheSize: number;
  httpCacheSize: number;
  codeCacheSize: number;
  serviceWorkerSize: number;
  indexedDbSize: number;
  localStorageSize: number;
  blobStorageSize: number;
  cachePath: string;
}

export interface UpdateInfo {
  version: string;
  releaseNotes?: string;
  releaseDate?: string;
}

export interface UpdateDownloadProgress {
  percent: number;
  transferred: number;
  total: number;
  bytesPerSecond: number;
}

export interface WindowState {
  isMaximized: boolean;
  isFullScreen: boolean;
  isVisible?: boolean;
}

export interface LocalTrackRecord {
  id: string;
  title: string;
  artist: string;
  album?: string;
  durationMs: number;
  fileName: string;
  audioFile: string;
  mimeType: string;
  coverFile?: string;
  byteSize: number;
  addedAt: number;
}

export interface LocalLibraryUsage {
  count: number;
  bytes: number;
}

export interface LocalImportResult {
  added: LocalTrackRecord[];
  duplicates: number;
  failed: string[];
}

export type SavedLyricsState = "found" | "missing";

export interface SavedTrackMeta {
  trackId: string;
  title: string;
  artists: string;
  album?: string;
  albumId?: string;
  artistId?: string;
  durationMs: number;
  coverUrl?: string;
  explicit?: boolean;
}

export interface SavedTrackRecord extends SavedTrackMeta {
  key: string;
  audioFile: string;
  mimeType: string;
  coverFile?: string;
  byteSize: number;
  savedAt: number;
  lyrics?: SavedLyricsState;
}

export interface SavedTrackInput {
  meta: SavedTrackMeta;
  audio: Uint8Array;
  mimeType: string;
  cover?: { data: Uint8Array; mimeType: string };
  lyricsState?: SavedLyricsState;
  lyrics?: unknown;
}

export interface SavedLibraryUsage {
  count: number;
  bytes: number;
  freeBytes: number | null;
}

export interface SavedTracksPartUsage {
  count: number;
  bytes: number;
}

export interface SignResult {
  signature?: string;
  timestamp?: number;
  nonce?: string;
  error?: string;
}

export interface SignInput {
  method: string;
  path: string;
  body?: string | null;
  timestamp?: number;
}

/** Mirrors `electron/netdiag.ts`; the Rust side returns the same JSON. */
export interface MainNetResult {
  dns: Array<{
    host: string;
    ok: boolean;
    addresses?: string[];
    resolvers?: string[];
    ms: number;
    error?: string;
  }>;
  tcp: Array<{
    host: string;
    port: number;
    ok: boolean;
    ms: number;
    error?: string;
  }>;
  tls: Array<{
    host: string;
    ok: boolean;
    protocol?: string;
    authorized?: boolean;
    subject?: string;
    issuer?: string;
    ms: number;
    error?: string;
  }>;
  controls: Array<{
    name: string;
    ok: boolean;
    status?: number;
    detail?: string;
    ms: number;
    error?: string;
  }>;
  trace: { host: string; ok: boolean; hops: string[]; ms: number; error?: string } | null;
  platform: string;
}

/** Mirrors `electron/netprobe/types.ts`. */
export interface NetProbeResult {
  internet: boolean;
  youtube: boolean;
  youtubeFailure: "timeout" | "blocked" | null;
  country: string | null;
}

export interface LinerNativeApi {
  platform: string;
  isHyprland: boolean;

  // ---- window chrome (inert on iOS) ----
  minimize: () => Promise<void>;
  toggleMaximize: () => Promise<void>;
  toggleFullScreen: () => Promise<void>;
  close: () => Promise<void>;
  isMaximized: () => Promise<boolean>;
  isFullScreen: () => Promise<boolean>;
  getRememberWindowBounds: () => Promise<boolean>;
  setRememberWindowBounds: (enabled: boolean) => Promise<boolean>;
  onWindowStateChange: (cb: (state: WindowState) => void) => () => void;
  onWindowRestored?: (cb: () => void) => () => void;
  onWindowMinimized?: (cb: () => void) => () => void;
  notePresence?: () => void;
  dragStart: () => void;
  dragMove: (deltaX: number, deltaY: number) => void;
  dragEnd: () => void;

  // ---- signing ----
  signRequest: (input: SignInput) => Promise<SignResult>;
  signMonitorRequest: (input: SignInput) => Promise<SignResult>;
  syncServerTime: (dateHeaderOrMs: string | number) => Promise<number>;
  setEdgeKey?: (key: string | null) => void;
  getServerTimeOffset: () => Promise<number>;
  signCoverUrl: (payload: string) => Promise<string>;
  signRawPayload: (payload: number[] | Uint8Array) => Promise<string>;

  // ---- auth ----
  authSession?: {
    get: () => StoredAuthSession | null;
    set: (
      next: { accessToken: string; refreshToken: string } | null,
    ) => Promise<StoredAuthSession | null>;
    refresh: (input: {
      apiBaseUrl: string;
      refreshToken?: string;
    }) => Promise<AuthRefreshResult>;
    onChange: (cb: (session: StoredAuthSession | null) => void) => () => void;
  };
  youtubeAuthSession?: {
    get: () => StoredAuthSession | null;
    set: (
      next: { accessToken: string; refreshToken: string } | null,
    ) => Promise<StoredAuthSession | null>;
  };

  // ---- misc ----
  onDeeplink: (cb: (target: DeeplinkTarget) => void) => () => void;
  /** Raw `liner://…` URL, from Siri, Shortcuts, widgets or another app. */
  onOpenUrl?: (cb: (url: string) => void) => () => void;
  diagnoseNetwork: (hosts: string[]) => Promise<MainNetResult>;
  probeNetwork: () => Promise<NetProbeResult>;
  openDownloads: (customPath?: string) => Promise<boolean>;
  openExportFolder: (customPath?: string) => Promise<boolean>;
  setAppIcon: (
    dataUrl: string,
    options?: { isDefault?: boolean },
  ) => Promise<void>;
  getAppVersion: () => Promise<string>;
  getCacheStats: () => Promise<LinerCacheStats>;
  openCacheFolder: () => Promise<boolean>;
  clearCoversCache: () => Promise<boolean>;
  clearAudioCache: (clearEntireDb?: boolean) => Promise<boolean>;
  clearHttpCache: () => Promise<boolean>;

  // ---- updater (inert: App Store / sideload updates are out of band) ----
  checkForUpdates: () => Promise<{
    available: boolean;
    version?: string;
    releaseNotes?: string;
    error?: string;
  }>;
  downloadUpdate: () => Promise<{ success: boolean; error?: string }>;
  quitAndInstall: () => Promise<boolean>;
  onUpdateAvailable: (cb: (info: UpdateInfo) => void) => () => void;
  onUpdateDownloadProgress: (
    cb: (progress: UpdateDownloadProgress) => void,
  ) => () => void;
  onUpdateDownloaded: (cb: (info: { version: string }) => void) => () => void;
  onUpdateError: (cb: (err: { message: string }) => void) => () => void;

  // ---- OTA frontend bundle (inert on iOS) ----
  bundleCheck: () => Promise<{
    available: boolean;
    manifest?: unknown;
    reason?: string;
  }>;
  bundleDownloadAndInstall: (
    manifest: unknown,
  ) => Promise<{ success: boolean; error?: string }>;
  bundleHotSwap: () => Promise<boolean>;
  bundleRollback: () => Promise<boolean>;
  bundleGetStatus: () => Promise<{
    appVersion: string;
    bundleVersion: string;
    isOta: boolean;
    sha256: string | null;
    effectivePath: string;
  }>;
  onBundleAvailable: (cb: (manifest: unknown) => void) => () => void;
  onBundleDownloadProgress: (
    cb: (progress: {
      percent: number;
      transferred: number;
      total: number;
    }) => void,
  ) => () => void;
  onBundleReady: (cb: (state: unknown) => void) => () => void;
  onBundleStamp: (cb: (line: string) => void) => () => void;

  // ---- touchbar / overlay (desktop only; inert) ----
  // The payloads are renderer-owned shapes (`TouchBarAction`, `OverlayAction`,
  // `OverlayStatePayload`). The desktop preload typed them as `any` because the
  // main process never inspects them; that contract is preserved deliberately so
  // the renderer's typed callbacks stay assignable.
  touchbarUpdateState: (state: any) => void;
  onTouchBarAction: (cb: (action: any) => void) => () => void;
  sendTouchBarAction: (action: any) => void;
  getTouchBarInitialState: () => Promise<any>;
  onTouchBarRequestSync: (cb: () => void) => () => void;
  overlayUpdateState: (state: any) => void;
  overlayReady?: () => void;
  onOverlayAction: (cb: (action: any) => void) => () => void;
  sendOverlayAction: (action: any) => void;
  toggleOverlay: () => Promise<boolean>;
  isOverlayOpen: () => Promise<boolean>;
  closeOverlay: () => void;
  onOverlayState: (cb: (state: any) => void) => () => void;
  getOverlayInitialState: () => Promise<any>;
  updateOverlaySettings: (settings: any) => void;
  pauseOverlayShortcuts: (paused: boolean) => void;
  getShortcutStatus?: () => Promise<Record<string, string>>;
  onShortcutStatus?: (
    cb: (status: Record<string, string>) => void,
  ) => () => void;
  onOverlaySettingsChanged: (cb: (settings: any) => void) => () => void;
  onOverlayVisibility: (cb: (visible: boolean) => void) => () => void;
  setOverlayAuthenticated?: (authenticated: boolean) => void;

  // ---- discord RPC (desktop only) ----
  discordUpdateState: (state: any) => void;
  discordUpdateSettings: (settings: any) => void;
  getDiscordStatus: () => Promise<DiscordStatus>;

  // ---- youtube helpers ----
  mintPoToken: (identifier: string) => Promise<string>;
  youtubeTvSession: () => Promise<any>;
  youtubeSolveN: (
    playerId: string,
    values: string[],
  ) => Promise<{ signatureTimestamp: number; values: string[] }>;

  // ---- libraries ----
  localTracksList: () => Promise<LocalTrackRecord[]>;
  localTracksImport: (paths: string[]) => Promise<LocalImportResult>;
  localTracksPick: () => Promise<LocalImportResult | null>;
  localTracksRemove: (ids: string[]) => Promise<void>;
  localTracksRemoveAll: () => Promise<number>;
  localTracksUsage: () => Promise<LocalLibraryUsage>;
  savedTracksList: () => Promise<SavedTrackRecord[]>;
  savedTracksSave: (input: SavedTrackInput) => Promise<SavedTrackRecord | null>;
  savedTracksRemove: (trackIds: string[]) => Promise<number>;
  savedTracksRemoveAll: () => Promise<number>;
  savedTracksUsage: () => Promise<SavedLibraryUsage>;
  savedTracksUsageOf: (trackIds: string[]) => Promise<SavedTracksPartUsage>;
  savedTracksOpenFolder: () => Promise<boolean>;
  savedTracksGetLyrics: (trackId: string) => Promise<unknown | null>;
  savedTracksSetLyrics: (
    trackId: string,
    state: SavedLyricsState,
    lyrics?: unknown,
  ) => Promise<boolean>;
  pathForFile: (file: File) => string;

  /**
   * Native Now Playing (lock screen, Dynamic Island, Siri).
   *
   * WKWebView playback is invisible to iOS, so the renderer publishes its
   * transport state here and receives remote-command events back.
   */
  nowPlaying?: {
    update: (state: {
      title: string;
      artist: string;
      album?: string | null;
      durationMs: number;
      positionMs: number;
      playing: boolean;
      artworkPath?: string | null;
    }) => Promise<void>;
    elapsed: (positionMs: number, rate: number) => Promise<void>;
    clear: () => Promise<void>;
  };
  /** System AirPlay / Bluetooth route sheet. */
  showRoutePicker?: () => Promise<boolean>;
  onRemoteCommand?: (cb: (command: string) => void) => () => void;
  onRemoteSeek?: (cb: (seconds: number) => void) => () => void;
}

declare global {
  interface Window {
    linerElectron?: LinerNativeApi;
  }
}
