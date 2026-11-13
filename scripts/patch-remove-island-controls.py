from pathlib import Path

# ---- 1. mobileSettings: drop the Dynamic Island key -------------------------
p = Path(r"D:\ios2\liner-music-ios\src\features\settings\store\mobileSettings.ts")
s = p.read_text(encoding="utf-8")
s = s.replace(
    """/**
 * Phone-only presentation preferences.
 *
 * `dynamicIsland` controls whether the app publishes Media Session metadata.
 * iOS renders that metadata in the Dynamic Island and on the lock screen, so
 * turning it off is how you keep Liner out of the island without losing
 * playback.
 */
export interface MobileSettings {
  /** Publish Now Playing metadata (Dynamic Island + lock screen controls). */
  dynamicIsland: boolean;
""",
    """/** Phone-only presentation preferences. */
export interface MobileSettings {
""",
)
s = s.replace(
    """const DEFAULTS: MobileSettings = {
  dynamicIsland: true,
  autoHideChrome: true,
  extraEdgeInset: true,
};""",
    """const DEFAULTS: MobileSettings = {
  autoHideChrome: true,
  extraEdgeInset: true,
};""",
)
s = s.replace(
    """    const parsed = JSON.parse(raw) as Partial<MobileSettings> & {
      islandPill?: boolean;
    };
    // migrate the earlier key
    if (parsed.islandPill !== undefined && parsed.dynamicIsland === undefined) {
      parsed.dynamicIsland = parsed.islandPill;
    }
    return { ...DEFAULTS, ...parsed };""",
    """    return { ...DEFAULTS, ...(JSON.parse(raw) as Partial<MobileSettings>) };""",
)
s = s.replace(
    """    const next: MobileSettings = {
      dynamicIsland: get().dynamicIsland,
      autoHideChrome: get().autoHideChrome,
      extraEdgeInset: get().extraEdgeInset,
      [key]: value,
    };""",
    """    const next: MobileSettings = {
      autoHideChrome: get().autoHideChrome,
      extraEdgeInset: get().extraEdgeInset,
      [key]: value,
    };""",
)
s = s.replace(
    """/** Read without React (player engine lives outside the tree). */
export function isDynamicIslandEnabled(): boolean {
  return useMobileSettings.getState().dynamicIsland;
}

""",
    "",
)
p.write_text(s, encoding="utf-8")

# ---- 2. playerRuntime: remove the gating ------------------------------------
p = Path(r"D:\ios2\liner-music-ios\src\features\player\engine\playerRuntime.ts")
s = p.read_text(encoding="utf-8")
s = s.replace(
    'import { isDynamicIslandEnabled } from "@/features/settings/store/mobileSettings";\n', ""
)
s = s.replace(
    """    if (!isDynamicIslandEnabled()) {
      // User opted out of the Dynamic Island / lock screen surface: keep the
      // session invisible instead of reporting playback.
      try {
        navigator.mediaSession.playbackState = "none";
      } catch {
        /* ignore */
      }
      return;
    }
""",
    "",
)
s = s.replace(
    """    if (!isDynamicIslandEnabled()) {
      // No metadata means no Now Playing entry, hence no island.
      navigator.mediaSession.metadata = null;
      return;
    }
""",
    "",
)
s = s.replace(
    "    if (!isDynamicIslandEnabled()) return;\n", ""
)
p.write_text(s, encoding="utf-8")

# ---- 3. MobileTab: drop the row ---------------------------------------------
p = Path(r"D:\ios2\liner-music-ios\src\features\settings\ui\tabs\MobileTab.tsx")
s = p.read_text(encoding="utf-8")
s = s.replace(
    'key: "dynamicIsland" | "autoHideChrome" | "extraEdgeInset"',
    'key: "autoHideChrome" | "extraEdgeInset"',
)
s = s.replace(
    """        {row(
          "dynamicIsland",
          "settings.mobile.dynamic_island.title",
          "settings.mobile.dynamic_island.description",
        )}
""",
    "",
)
s = s.replace(
    """          onClick={() => {
            const next = !settings[key];
            settings.set(key, next);
            // Toggling this off must clear the published metadata now,
            // otherwise iOS keeps showing the current track in the island.
            if (key === "dynamicIsland" && !next && typeof navigator !== "undefined" && "mediaSession" in navigator) {
              try {
                navigator.mediaSession.metadata = null;
                navigator.mediaSession.playbackState = "none";
              } catch {
                /* ignore */
              }
            }
          }}""",
    "          onClick={() => settings.set(key, !settings[key])}",
)
p.write_text(s, encoding="utf-8")

# ---- 4. AppFrame: drop the fullscreen window controls ------------------------
p = Path(r"D:\ios2\liner-music-ios\src\features\navigation\ui\AppFrame.tsx")
s = p.read_text(encoding="utf-8")
start = s.find("      {isFullscreenPlayer && (")
if start != -1:
    end = s.find("      )}", start)
    if end != -1:
        s = s[:start] + s[end + len("      )}\n") :]
p.write_text(s, encoding="utf-8")

print("removals applied")
