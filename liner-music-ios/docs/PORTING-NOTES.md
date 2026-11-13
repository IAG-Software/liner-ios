# Porting notes — Liner desktop (Electron) → Liner Music (Tauri v2 / iOS)

This document records every architectural decision that is *not* obvious from
reading the code, plus the places where the port deliberately differs from
`tryliner/desktop`.

Upstream baseline: `tryliner/desktop` @ `197179b` (v0.2.12).

---

## 1. What was actually ported

| Layer | Desktop | iOS port | Status |
| --- | --- | --- | --- |
| Renderer | React 19 + Tailwind v4 in Chromium | **same bundle**, byte-identical sources | ✅ full |
| IPC | `ipcMain` / `ipcRenderer` (135 call sites) | one adapter object backed by `#[tauri::command]` | ✅ full |
| Signing | `signer/` compiled to WASM, run in renderer | `signer/` vendored, linked natively | ✅ full |
| Audio | HTML `<audio>` + WebAudio graph | Rust: symphonia → DSP → cpal/CoreAudio | ✅ native |
| Lock screen | none on Linux/Windows | `MPNowPlayingInfoCenter` + `MPRemoteCommandCenter` | ✅ new |
| Cache | `app.getPath('userData')` + fs | `Library/Caches` via `BaseDirectory::AppCache` | ✅ full |
| Saved/local library | Node `fs` in main process | Rust, `Application Support` | ✅ full |
| Window chrome | custom frame, drag, maximise | OS window | ⛔ N/A on iOS |
| Touch Bar / overlay window | separate `BrowserWindow` | not applicable | ⛔ N/A |
| Discord RPC | local socket (`@xhayper/discord-rpc`) | sandbox blocks sockets to other apps | ⛔ N/A |
| OTA frontend bundle | downloads + hot-swaps `dist` | bundle is sealed in the `.ipa` | ⛔ by design |
| Auto-update | `electron-updater` | sideload tools own updates | ⛔ by design |
| YouTube PoToken / TV session | Node worker + BotGuard | needs a Worker with raw JS eval | ⚠️ degraded |

The renderer rewrite was avoided on purpose: the app is 514 `.ts` + 233 `.tsx`
files and the bridge surface is a single well-typed object. Re-implementing that
object (≈600 lines) is a far smaller and safer change than editing 135 call
sites, and it keeps the port mergeable with upstream.

---

## 2. IPC channel → Tauri command map

Every Electron channel becomes a snake_case command. The renderer never sees
these names; `src/tauri/bridge.ts` is the only translation layer.

### Window (`window:*`) — inert on iOS

| Electron channel | Tauri command | iOS behaviour |
| --- | --- | --- |
| `window:minimize` | `window_minimize` | no-op |
| `window:toggle-maximize` | `window_toggle_maximize` | no-op |
| `window:toggle-fullscreen` | `window_toggle_fullscreen` | no-op |
| `window:close` | `window_close` | no-op |
| `window:is-maximized` | `window_is_maximized` | `false` |
| `window:is-fullscreen` | `window_is_fullscreen` | `false` |
| `window:get-remember-bounds` | `window_get_remember_bounds` | `true` |
| `window:set-remember-bounds` | `window_set_remember_bounds` | `true` |
| `window:presence` | `window_presence` | no-op |
| `window:drag-start/-move/-end` | `window_drag_start/-move/-end` | no-op |

### Signing (`signer:*`, `edge:*`)

| Electron channel | Tauri command | Notes |
| --- | --- | --- |
| `signer:sign-request` | `signer_sign_request` | HMAC-SHA256 over `METHOD\nPATH\nTS\nNONCE\nSHA256(BODY)` |
| `signer:sign-monitor-request` | `signer_sign_monitor_request` | separate monitor key |
| `signer:sync-time` | `signer_sync_time` | accepts `Date` header or epoch ms |
| `signer:get-time-offset` | `signer_get_time_offset` | |
| `signer:sign-cover-url` | `signer_sign_cover_url` | |
| `signer:sign-raw-payload` | `signer_sign_raw_payload` | |
| `edge:set-key` | `edge_set_key` | held in a native `Mutex<String>` |

### Auth (`auth:*`, `youtube-auth:*`)

| Electron channel | Tauri command | Notes |
| --- | --- | --- |
| `auth:get-session` (**`sendSync`**) | `auth_get_session` | value is prefetched at boot, see §4 |
| `auth:set-session` | `auth_set_session` | emits `auth:session-changed` |
| `auth:refresh` | `auth_refresh` | single-flight, rotating-token aware |
| `youtube-auth:get-session` | `youtube_auth_get_session` | |
| `youtube-auth:set-session` | `youtube_auth_set_session` | |

### Storage / shell

| Electron channel | Tauri command | iOS behaviour |
| --- | --- | --- |
| `storage:get-cache-stats` | `storage_get_cache_stats` | real sizes from `Library/Caches` |
| `storage:clear-covers-cache` | `storage_clear_covers_cache` | |
| `storage:clear-audio-cache` | `storage_clear_audio_cache` | |
| `storage:clear-http-cache` | `storage_clear_http_cache` | |
| `shell:open-cache-folder` | `storage_open_cache_folder` | `false` (no Files UI) |
| `shell:open-downloads` | `shell_open_downloads` | `false` |
| `shell:open-export-folder` | `shell_open_export_folder` | `false` |

### Libraries

| Electron channel | Tauri command |
| --- | --- |
| `local-tracks:*` | `local_tracks_list/import/pick/remove/remove_all/usage` |
| `saved-tracks:*` | `saved_tracks_list/save/remove/remove_all/usage/usage_of/open_folder/get_lyrics/set_lyrics` |

### App / net

| Electron channel | Tauri command | Notes |
| --- | --- | --- |
| `app:get-version` | `app_get_version` | |
| `app:set-icon` | `app_set_icon` | no-op; icons are in the bundle |
| `net:diagnose` | `net_diagnose` | TCP:443 reachability per host |
| `net:probe` | `net_probe` | one HTTPS round trip |

### Not implemented (desktop-only surfaces)

`updater:*`, `bundle:*`, `touchbar:*`, `overlay:*`, `discord*` — the bridge
returns the "nothing happened" value so feature detection in the renderer falls
through cleanly instead of throwing.

---

## 3. Apple media stack: why a C shim instead of `objc2`

The brief suggested `objc2-avf-audio` / `objc2-media-player`. The port uses a
9-function C shim (`ios/LinerMusic/LinerPlatform.m`) instead. This is a
deliberate deviation, and the reasoning is worth recording:

1. **Ownership.** `MPRemoteCommandCenter` handlers are blocks. Keeping them
   alive from Rust means hand-managing `retain`/`release` on objects that
   outlive the call — untestable without a device, and a leak class of bug.
2. **No Swift.** Either approach avoids `swiftc`, but a `.m` file compiles with
   the *same* clang invocation that links the Rust archive, so the build stays a
   single toolchain.
3. **Audit surface.** The ABI is 9 functions; everything else stays in Rust.

The Rust side (`src-tauri/src/platform_ios.rs`) declares the externs and is the
only place that knows the shim exists. Swapping to `objc2` later touches that
file only.

Exported C ABI:

```c
int    liner_audio_session_activate(void);
void   liner_audio_session_deactivate(void);
void   liner_now_playing_update(const char*, const char*, const char*, double, double, double, const char*);
void   liner_now_playing_elapsed(double, double);
void   liner_now_playing_rate(double);
void   liner_now_playing_playing(int);
void   liner_now_playing_clear(void);
void   liner_remote_commands_install(void (*)(int));
double liner_take_remote_seek(void);
int    liner_open_url(const char*);
void   liner_idle_timer_disabled(int);
```

Threading rule: the two callbacks are invoked on the **main thread** and only
latch an atomic. The audio worker polls them (`Worker::poll_remote`), so there
is exactly one state machine and ObjC never touches the audio thread.

---

## 4. Synchronous IPC → boot-time prefetch

`authSession.get()` is called synchronously during the first React render.
Electron offered `ipcRenderer.sendSync`; Tauri does not.

`installLinerBridge()` therefore fetches the two sessions *before* mounting and
keeps them in module scope; every later mutation arrives through
`auth:session-changed`. `main.tsx` awaits the bridge installation before
`createRoot(...).render(...)`.

---

## 5. Audio architecture

```
Tauri command (tokio)          audio worker (std::thread)        CoreAudio
─────────────────────          ──────────────────────────        ─────────
cache.ensure_audio()  ──cmd──▶ symphonia decode
                               → cubic resample to 48 kHz
                               → 10-band EQ + soft limiter
                               → SPSC ring (1.5 s)  ───────────▶ render callback
                                 ▲                                 │
                                 └──── position / underrun ◀────────┘
```

* **Why file-based and not streaming:** the cache manager materialises media
  first. This gives free seeking, exact duration, gapless pre-buffering, and no
  rebuffering when iOS suspends a socket in the background. It also matches what
  the desktop app already did with its audio cache.
* **Why one fixed 48 kHz pipeline:** tearing down the `AudioUnit` per track
  would deactivate the audio session and drop lock-screen state. 48 kHz is the
  native rate of every iPhone output route.
* **Why the ring is lock-free:** the render callback must never block. The
  producer/consumer pair uses release/acquire indices over a flat `f32` buffer.
* **Gapless:** the next queue entry is decoded 4 s early (`maybe_preload_next`)
  and swapped in at EOF, so there is no silence at the boundary.

Position reported to the UI is the *audible* position: decoded frames minus what
is still sitting in the ring. That is what the lyric clock is driven from.

---

## 6. Lyrics

The renderer already parses richsync/TTML/LRC into `SyncedLine[]`. That structure
is posted once to `lyrics_load`, compiled into a flat timeline (binary search by
start time), and driven by the audio worker's sample position.

Events (emitted at most 5×/s, only on change):

* `lyrics:line` — active line changed
* `lyrics:word` — active syllable/word changed (with `progress` 0..1)
* `lyrics:progress` — every tick, includes `frame` so the UI can re-anchor

A `setInterval` in the WebView was rejected: timers stall when the app is
backgrounded and drift against CoreAudio under load.

---

## 7. Storage layout

```
Library/Caches/com.iagsoftware.linermusic/     ← purgeable
  audio/<sha256(url)[:32]>.<ext>
  covers/<sha256(url)[:32]>.<jpg|png|webp>
  http/

Library/Application Support/                    ← persistent, backed up
  auth/session.enc          AES-256-GCM envelope
  auth/device.key           32-byte key, 0600
  youtube-auth/session.enc
  local/library.json        imported tracks
  local/audio/<uuid>.<ext>
  saved/library.json        saved tracks
  saved/audio| covers| lyrics/
```

### Session encryption, stated honestly

The desktop build used Electron `safeStorage` (OS keyring). The port writes an
AES-256-GCM envelope whose key lives in a separate `0600` file in the same
container. That protects a stolen *backup* and casual inspection; it does **not**
protect against an attacker who can already read the app container, because the
key is adjacent. Moving the key to the iOS Keychain is the documented upgrade
path and does not change the envelope format.

---

## 8. Signing: WASM → native

`signer/` is vendored verbatim at `src-tauri/crates/liner-signer`. On desktop it
is compiled to WASM and loaded by the renderer; here it is linked into the
static library.

Consequence: the obfuscated HMAC keys are no longer a script asset that can be
lifted out of the bundle with `strings`. They still exist in the binary — this
is a raise-the-bar measure, not a secret.

Canonical string (unchanged from `signer/src/ffi.rs`):

```
METHOD \n PATH \n TIMESTAMP \n NONCE \n SHA256_HEX(BODY)
```

---

## 9. Known gaps

1. **YouTube PoToken.** `mintPoToken` needs a BotGuard JS worker on desktop.
   The bridge throws, and the renderer's `InnertubeError(501, …)` path applies.
   YouTube playback therefore degrades; SoundCloud/local/saved playback is
   unaffected.
2. **YouTube TV session.** Explicitly desktop-only upstream (`electronApi()`
   throws). Same behaviour here.
3. **`pathForFile`.** No filesystem paths exist in the sandbox; returns the file
   name. Only used by desktop drag-and-drop.
4. **iOS 27 SDK.** Built against iPhoneOS 16.5 stubs (the newest public set). A
   binary linked against 16.5 runs on iOS 27; targeting the 27.0 SDK requires
   Xcode 27's SDK and is a one-line change (`IPHONEOS_DEPLOYMENT_TARGET`).
5. **No notarisation / provisioning.** The `.ipa` is unsigned by design; it is
   structured for SideStore / AltStore / TrollStore re-signing.
