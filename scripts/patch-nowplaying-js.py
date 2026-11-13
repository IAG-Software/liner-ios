from pathlib import Path

# ---- api.ts: types ---------------------------------------------------------
p = Path(r"D:\ios2\liner-music-ios\src\tauri\api.ts")
s = p.read_text(encoding="utf-8")
s = s.replace(
    "  pathForFile: (file: File) => string;\n}",
    """  pathForFile: (file: File) => string;

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
  onRemoteCommand?: (cb: (command: string) => void) => () => void;
  onRemoteSeek?: (cb: (seconds: number) => void) => () => void;
}""",
)
p.write_text(s, encoding="utf-8")

# ---- bridge.ts: implementation ---------------------------------------------
p = Path(r"D:\ios2\liner-music-ios\src\tauri\bridge.ts")
s = p.read_text(encoding="utf-8")
s = s.replace(
    "    pathForFile: (file: File) => file.name,\n  };",
    """    pathForFile: (file: File) => file.name,

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
    onRemoteCommand: (cb: (command: string) => void) =>
      subscribe<string>("media:remote", cb),
    onRemoteSeek: (cb: (seconds: number) => void) =>
      subscribe<number>("media:remote-seek", cb),
  };""",
)
p.write_text(s, encoding="utf-8")
print("bridge + api patched")
