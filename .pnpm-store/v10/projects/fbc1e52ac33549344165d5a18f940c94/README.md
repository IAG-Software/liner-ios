# Liner Music — iOS

A port of [Liner desktop](https://github.com/tryliner/desktop) (Electron + React 19)
to **iOS arm64**, built with **Rust + Tauri v2** and cross-compiled from Windows —
no macOS, no Xcode, no Apple developer certificate. Output: a self-contained,
unsigned `.ipa` for SideStore / AltStore / TrollStore.

```
      Windows host                     WSL2 (Ubuntu)                iPhone
 ┌────────────────────┐          ┌──────────────────────┐      ┌──────────┐
 │ pnpm build  → dist │  rsync   │ cargo (aarch64-ios)  │ ipa  │ WKWebView│
 │                    │ ───────▶ │ clang + ld64.lld     │ ───▶ │ + CoreAudio
 │ scripts/*.ps1      │          │ + iPhoneOS SDK       │      │          │
 └────────────────────┘          └──────────────────────┘      └──────────┘
```

---

## 1. What is in the box

| | |
| --- | --- |
| Renderer | The **unmodified** upstream React 19 + Tailwind v4 app (7 680 modules). |
| Bridge | `src/tauri/bridge.ts` re-creates Electron's `window.linerElectron` object on top of Tauri commands. |
| Native core | `src-tauri/` — 20 Rust modules: audio engine, lyrics scheduler, cache, libraries, signing, auth, diagnostics. |
| Host app | `ios/LinerMusic/` — a 30-line `main.mm` plus an Objective-C platform shim. |
| Build | `scripts/` — WSL cross-compile + IPA assembly, driven from PowerShell. |

### Why the renderer is untouched

Upstream talks to the main process through one `contextBridge` object at 135 call
sites. Re-implementing that object (≈600 lines) is smaller, safer and far more
mergeable than editing 135 call sites across 747 files. See
[`docs/PORTING-NOTES.md`](docs/PORTING-NOTES.md) §1.

---

## 2. Directory tree

```
liner-music-ios/
├── index.html                     # viewport-fit=cover, safe-area vars, 120 Hz hints
├── package.json                   # renderer deps (Electron tooling removed)
├── vite.config.ts                 # base:"./" for the tauri:// origin, rolldown chunks
├── tsconfig.json
│
├── src/                           # ── renderer (upstream, unchanged) ──────────
│   ├── main.tsx                   # installs the native bridge, then renders
│   ├── tauri/
│   │   ├── api.ts                 # LinerNativeApi — the bridge contract
│   │   └── bridge.ts              # window.linerElectron  →  invoke()/listen()
│   ├── app/  entities/  features/  widgets/  shared/  languages/
│   └── electron.d.ts              # points the old bridge type at src/tauri/api
│
├── electron/                      # ── type-only shims ─────────────────────────
│   ├── preload.ts                 # re-exports LinerNativeApi as LinerElectronApi
│   ├── local-tracks/types.ts
│   ├── saved-tracks/types.ts
│   ├── netdiag.ts
│   └── netprobe/types.ts
│
├── src-tauri/                     # ── Rust native core ─────────────────────────
│   ├── Cargo.toml                 # staticlib + cdylib + rlib
│   ├── build.rs                   # compiles the ObjC shim into the crate
│   ├── tauri.conf.json
│   ├── tauri.ios.conf.json        # iOS-27 / SDK-specific overrides
│   ├── icons/
│   ├── crates/liner-signer/       # vendored from upstream `signer/`
│   └── src/
│       ├── lib.rs                 # mobile entry point, plugin/state wiring
│       ├── main.rs                # desktop/dev binary
│       ├── audio_engine.rs        # transport state machine, gapless queue
│       ├── audio/
│       │   ├── decode.rs          # symphonia → interleaved f32 stereo
│       │   ├── dsp.rs             # 10-band biquad EQ + soft limiter
│       │   ├── output.rs          # cpal → CoreAudio render callback
│       │   ├── resample.rs        # Catmull-Rom SRC to 48 kHz
│       │   └── ring.rs            # lock-free SPSC sample ring
│       ├── lyrics_sync.rs         # syllable timeline + Tauri events
│       ├── cache_manager.rs       # Library/Caches audio + artwork
│       ├── local_library.rs       # imported files, tag probing
│       ├── saved_library.rs       # offline library, sharded on disk
│       ├── auth.rs                # AES-GCM session envelopes + refresh
│       ├── signer.rs              # native HMAC (was WASM)
│       ├── media_protocol.rs      # liner-local:// with HTTP range support
│       ├── platform_ios.rs        # extern "C" bindings to the ObjC shim
│       ├── state.rs  paths.rs  jsonstore.rs  net.rs  error.rs
│       └── commands/              # 10 modules, ~70 #[tauri::command] handlers
│
├── ios/LinerMusic/                # ── UIKit host ──────────────────────────────
│   ├── main.mm                    # calls start_app() (Tauri's C entry point)
│   ├── LinerPlatform.m            # AVAudioSession, MPRemoteCommandCenter,
│   │                              # MPNowPlayingInfoCenter, UIDocumentPicker
│   ├── LinerTauriSwiftShim.m      # C impls of Tauri's Swift-API symbols
│   ├── Info.plist                 # UIBackgroundModes: audio, ATS, icons
│   └── icons/                     # loose-file icon set (no actool needed)
│
├── scripts/
│   ├── build-ios.ps1              # Windows entry point
│   ├── setup-toolchain.ps1        # one-time WSL provisioning
│   ├── ios-build.sh               # compile → link → bundle → zip
│   ├── wsl-setup-rust.sh
│   └── make-ios-icons.py
│
└── docs/PORTING-NOTES.md          # IPC map, audio design, known gaps
```

---

## 3. Build

### 3.1 Prerequisites (one time)

* Windows 10/11 with **WSL2** and an Ubuntu 24.04 distro
  (`wsl --install -d Ubuntu-24.04`)
* Node 22+ and pnpm on Windows

```powershell
pwsh -File scripts/setup-toolchain.ps1
```

This installs, inside WSL, everything under `/opt` so nothing leaks onto the host:

| Component | Path |
| --- | --- |
| clang / ld64.lld | apt (`clang`, `lld`) |
| Rust + `aarch64-apple-ios` | `/opt/rust` |
| iPhoneOS SDK (16.5 stubs) | `/opt/ios/sdk-src/iPhoneOS16.5.sdk` |

### 3.2 Build the IPA

```powershell
pwsh -File scripts/build-ios.ps1
# → release/LinerMusic-unsigned-ios.ipa
```

Rust-only iteration (skip the renderer build):

```powershell
pwsh -File scripts/build-ios.ps1 -SkipFrontend
```

### 3.3 Manual equivalent

```bash
pnpm install && pnpm build                    # host
bash scripts/ios-build.sh                     # WSL
```

The script performs six steps and prints each one:
`sync → cargo build → clang objects → ld64.lld link → bundle → zip`.

Key environment it sets (identical to what xtool establishes):

```bash
SDKROOT=/opt/ios/sdk-src/iPhoneOS16.5.sdk
CC_aarch64_apple_ios="clang -target arm64-apple-ios16.5 -isysroot $SDKROOT -arch arm64"
AR_aarch64_apple_ios=llvm-ar
CARGO_TARGET_AARCH64_APPLE_IOS_LINKER=/opt/ios/bin/ios-linker   # clang -fuse-ld=lld
```

### 3.4 Targeting a newer SDK (iOS 27)

The public stub set tops out at 16.5; a binary linked against 16.5 **runs** on
iOS 27. To build against the real 27.0 SDK (Xcode 27 / xtool's SDK bundle):

```bash
SDKROOT=/path/to/iPhoneOS27.0.sdk IPHONEOS_DEPLOYMENT_TARGET=27.0 \
  bash scripts/ios-build.sh
```

and set `bundle.iOS.minimumSystemVersion` to `"27.0"` in `tauri.conf.json`
(or use the overlay in `tauri.ios.conf.json`).

---

## 4. Installing the unsigned IPA

The `.ipa` is intentionally unsigned and has **no** provisioning profile, so it
must be re-signed by a sideloading tool:

| Tool | Notes |
| --- | --- |
| **SideStore / AltStore** | Drag the `.ipa` in; it re-signs with your Apple ID and refreshes weekly. |
| **TrollStore** | Installs directly on supported iOS versions (permanent signature). |
| **Xcode / Apple Configurator** | Only with a paid team; not required. |

Bundle id: `com.iagsoftware.linermusic`. To keep SideStore happy across
refreshes, keep the bundle id stable.

---

## 5. Verifying the artefact

```bash
unzip -l LinerMusic-unsigned-ios.ipa
# Payload/LinerMusic.app/LinerMusic        (~25 MB, Mach-O 64-bit arm64)
# Payload/LinerMusic.app/Info.plist
# Payload/LinerMusic.app/AppIcon60x60@2x.png ...
```

```bash
file Payload/LinerMusic.app/LinerMusic
# Mach-O 64-bit arm64 executable
```

The renderer is embedded in the binary by `tauri-codegen`, so there is no
`web/` directory in the bundle.

---

## 6. Layout notes for iOS 27 devices

* `viewport-fit=cover` + `env(safe-area-inset-*)` — the player bar clears the
  home indicator, the header clears the Dynamic Island.
* `-webkit-overflow-scrolling: touch` and layer promotion on scroll containers
  let the compositor drive scrolling at the display's rate on ProMotion panels.
* `UIBackgroundModes: audio` + `AVAudioSessionCategoryPlayback` — playback and
  lock-screen metadata survive app suspension.
* `UIFileSharingEnabled` — you can drop audio into Liner's Documents folder from
  the Files app and import it from the local library.

---

## 7. Licence

GPL-3.0, inherited from upstream Liner.
