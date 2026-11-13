from pathlib import Path

# ---- 1. boot the bridge ----------------------------------------------------
p = Path(r"D:\ios2\liner-music-ios\src\app\mainWindowServices.ts")
s = p.read_text(encoding="utf-8")
s = s.replace(
    'import { initOfflineMode } from "@/features/connectivity/engine/offlineController";',
    'import { initOfflineMode } from "@/features/connectivity/engine/offlineController";\n'
    'import { installNativeMediaBridge } from "@/features/player/engine/nativeMediaBridge";',
)
s = s.replace(
    "  registerSignOutTask(forgetStorageAnalytics);",
    "  registerSignOutTask(forgetStorageAnalytics);\n"
    "  // Lock screen / Dynamic Island / Siri transport, forwarded from the native\n"
    "  // Now Playing centre into the web player.\n"
    "  installNativeMediaBridge();",
)
p.write_text(s, encoding="utf-8")

# ---- 2. publish state from the runtime -------------------------------------
p = Path(r"D:\ios2\liner-music-ios\src\features\player\engine\playerRuntime.ts")
s = p.read_text(encoding="utf-8")

# module-level track memory so the helper does not need the store
s = s.replace(
    'import { isDynamicIslandEnabled } from "@/features/settings/store/mobileSettings";\n',
    "",
)
if "let nativeNowPlayingTrack" not in s:
    anchor = "export class PlayerRuntime"
    idx = s.find(anchor)
    if idx == -1:
        # fall back to the first class/export marker
        idx = s.find("\n\n", s.index("\n\n") + 2)
    helper = """
// Last track handed to the native Now Playing centre. Kept at module scope so
// the publish helper does not need the zustand store.
let nativeNowPlayingTrack: Track | null = null;

/**
 * Mirror transport state into `MPNowPlayingInfoCenter`.
 *
 * WKWebView media is invisible to iOS, so this is what makes the lock screen,
 * the Dynamic Island and Siri aware of playback at all.
 */
function publishNativeNowPlaying(
  audio: HTMLAudioElement,
  track: Track | null,
  opts: { positionOnly?: boolean } = {},
): void {
  if (typeof window === "undefined") return;
  const api = window.linerElectron?.nowPlaying;
  if (!api) return;

  if (!track) {
    nativeNowPlayingTrack = null;
    void api.clear();
    return;
  }

  nativeNowPlayingTrack = track;

  const durationMs = Number.isFinite(audio.duration)
    ? Math.round(audio.duration * 1000)
    : (track.durationMs ?? 0);
  const positionMs = Number.isFinite(audio.currentTime)
    ? Math.round(audio.currentTime * 1000)
    : 0;
  const playing = !audio.paused;

  if (opts.positionOnly) {
    void api.elapsed(positionMs, playing ? 1 : 0);
    return;
  }

  void api.update({
    title: track.title,
    artist: track.artists,
    album: (track as { albumTitle?: string }).albumTitle ?? null,
    durationMs,
    positionMs,
    playing,
  });
}
"""
    s = s[:idx] + helper + "\n" + s[idx:]

# metadata: remember the track and publish
s = s.replace(
    """    } catch {
      // ignore metadata assignment failures
    }
  }
""",
    """    } catch {
      // ignore metadata assignment failures
    }

    publishNativeNowPlaying(this.audio, track);
  }
""",
    1,
)
s = s.replace(
    """    if (!track) {
      navigator.mediaSession.metadata = null;
      return;
    }""",
    """    if (!track) {
      navigator.mediaSession.metadata = null;
      publishNativeNowPlaying(this.audio, null);
      return;
    }""",
)

# state changes: republish playing flag
s = s.replace(
    """      if (status === "idle") navigator.mediaSession.metadata = null;
    } catch {
      // ignore unsupported environments
    }
  }""",
    """      if (status === "idle") navigator.mediaSession.metadata = null;
    } catch {
      // ignore unsupported environments
    }

    if (status === "idle") publishNativeNowPlaying(this.audio, null);
    else publishNativeNowPlaying(this.audio, nativeNowPlayingTrack);
  }""",
)

# position ticks: cheap elapsed-only update
s = s.replace(
    """      });
    } catch {
      // ignore invalid transient states
    }
  }""",
    """      });
    } catch {
      // ignore invalid transient states
    }

    publishNativeNowPlaying(this.audio, nativeNowPlayingTrack, {
      positionOnly: true,
    });
  }""",
)

p.write_text(s, encoding="utf-8")
print("runtime publishes now playing")
