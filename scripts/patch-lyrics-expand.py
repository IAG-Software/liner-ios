from pathlib import Path

p = Path(r"D:\ios2\liner-music-ios\src\features\player\ui\FullscreenPlayer.tsx")
s = p.read_text(encoding="utf-8")
o = s

# controls become an overlay on phones, so the lyrics own the full height
s = s.replace(
    '''            isMobile
              ? "order-2 shrink-0 pb-2"
              : "flex-1 items-center justify-center gap-5",''',
    '''            isMobile
              ? "order-2 absolute inset-x-0 bottom-0 z-20 pb-2"
              : "flex-1 items-center justify-center gap-5",''',
)

# and the lyrics reclaim that space whenever the chrome collapses
s = s.replace(
    '''        <div
          className={cn(
            "min-h-0 flex flex-col relative overflow-hidden",
            isMobile
              ? "order-1 flex-1 py-2"''',
    '''        <div
          style={
            isMobile
              ? {
                  paddingBottom: chromeVisible ? 96 : 0,
                  transition: "padding-bottom 300ms cubic-bezier(0.22,1,0.36,1)",
                }
              : undefined
          }
          className={cn(
            "min-h-0 flex flex-col relative overflow-hidden",
            isMobile
              ? "order-1 flex-1 py-2"''',
)

assert s != o, "no changes"
p.write_text(s, encoding="utf-8")
print("fullscreen overlay patched")
