from pathlib import Path

p = Path(r"D:\ios2\liner-music-ios\src\features\player\engine\playerRuntime.ts")
s = p.read_text(encoding="utf-8")
o = s

# import the flag (module-level import, no React involved)
anchor = 'import { playerStore as store } from "../store/playerStore";'
if anchor in s:
    s = s.replace(
        anchor,
        anchor
        + '\nimport { isDynamicIslandEnabled } from "@/features/settings/store/mobileSettings";',
        1,
    )
else:
    # fall back to the first import line
    first_nl = s.index("\n")
    s = (
        s[: first_nl + 1]
        + 'import { isDynamicIslandEnabled } from "@/features/settings/store/mobileSettings";\n'
        + s[first_nl + 1 :]
    )

s = s.replace(
    '''  private syncMediaSessionState(status: string) {
    if (typeof navigator === "undefined" || !("mediaSession" in navigator))
      return;
    try {''',
    '''  private syncMediaSessionState(status: string) {
    if (typeof navigator === "undefined" || !("mediaSession" in navigator))
      return;
    if (!isDynamicIslandEnabled()) {
      // User opted out of the Dynamic Island / lock screen surface: keep the
      // session invisible instead of reporting playback.
      try {
        navigator.mediaSession.playbackState = "none";
      } catch {
        /* ignore */
      }
      return;
    }
    try {''',
)

s = s.replace(
    '''  public updateMediaSessionMetadata(track: Track | null) {
    if (typeof navigator === "undefined" || !("mediaSession" in navigator))
      return;
    if (!track) {''',
    '''  public updateMediaSessionMetadata(track: Track | null) {
    if (typeof navigator === "undefined" || !("mediaSession" in navigator))
      return;
    if (!isDynamicIslandEnabled()) {
      // No metadata means no Now Playing entry, hence no island.
      navigator.mediaSession.metadata = null;
      return;
    }
    if (!track) {''',
)

s = s.replace(
    '''  private syncMediaSessionPosition() {
    if (typeof navigator === "undefined" || !("mediaSession" in navigator))
      return;
    const ms = navigator.mediaSession as MediaSession & {''',
    '''  private syncMediaSessionPosition() {
    if (typeof navigator === "undefined" || !("mediaSession" in navigator))
      return;
    if (!isDynamicIslandEnabled()) return;
    const ms = navigator.mediaSession as MediaSession & {''',
)

assert s != o, "no changes"
p.write_text(s, encoding="utf-8")
print("playerRuntime gated")
