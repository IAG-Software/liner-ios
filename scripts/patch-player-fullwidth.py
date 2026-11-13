from pathlib import Path

p = Path(r"D:\ios2\liner-music-ios\src\features\navigation\ui\AppFrame.tsx")
s = p.read_text(encoding="utf-8")
o = s

# unused after removing the fullscreen window controls
s = s.replace('import WindowControlsNotch from "./WindowControlsNotch";\n', "")
if "glassBlur" in s and s.count("glassBlur") == 1:
    s = s.replace("  const glassBlur = useGlassBlur();\n", "")

# bottom safe-area on phones so the player clears the home indicator
s = s.replace(
    '        extraEdgeInset ? "max-sm:px-3 max-sm:pb-3" : ""',
    '        extraEdgeInset\n'
    '          ? "max-sm:px-3 max-sm:pb-[calc(env(safe-area-inset-bottom,0px)+10px)]"\n'
    '          : "max-sm:pb-[calc(env(safe-area-inset-bottom,0px)+6px)]"',
)

# keep the desktop placement, add a full-width phone placement
s = s.replace(
    "          {shouldShowMiniPlayer && (\n            <div\n              ref={miniPlayerContainerRef}",
    "          {shouldShowMiniPlayer && !isMobile && (\n            <div\n              ref={miniPlayerContainerRef}",
)

mobile_player = """
      {/* Phone: the player spans the full width, under the rail too — that is
          the empty corner it used to leave, and it gives the title room. */}
      {shouldShowMiniPlayer && isMobile && (
        <div ref={miniPlayerContainerRef} className="relative z-[55] w-full shrink-0">
          <MiniPlayer
            embedded
            onQueueOpenChange={handleQueuePopupToggle}
            onQueueToggle={handleQueuePopupToggleRequest}
            onFullscreenOpen={openFullscreenPlayer}
          />
        </div>
      )}

      <AppOverlays />"""

s = s.replace("\n      <AppOverlays />", mobile_player, 1)

assert s != o, "no changes"
p.write_text(s, encoding="utf-8")
print("appframe layout patched")
