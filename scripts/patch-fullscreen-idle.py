from pathlib import Path

p = Path(r"D:\ios2\liner-music-ios\src\features\player\ui\FullscreenPlayer.tsx")
s = p.read_text(encoding="utf-8")
o = s

s = s.replace(
    'import { useCallback, useEffect, useState } from "react";',
    'import { useCallback, useEffect, useRef, useState } from "react";',
)
s = s.replace(
    'import { useIsMobile } from "@/shared/hooks/useIsMobile";',
    'import { useIsMobile } from "@/shared/hooks/useIsMobile";\n'
    'import {\n  CHROME_IDLE_MS,\n  useMobileSettings,\n} from "@/features/settings/store/mobileSettings";',
)
s = s.replace(
    'const { islandPill, autoHideChrome } = useMobileSettings();',
    'const { autoHideChrome } = useMobileSettings();',
)

s = s.replace(
    '      data-theme="dark"\n'
    '      className="dark relative h-full w-full overflow-hidden bg-black text-white"\n'
    '    >',
    '      data-theme="dark"\n'
    '      className="dark relative h-full w-full overflow-hidden bg-black text-white"\n'
    '      onPointerDown={bumpChrome}\n'
    '    >',
)

s = s.replace(
    '      <div className="absolute top-[calc(6px+env(safe-area-inset-top,0px))] left-1/2 z-30 flex -translate-x-1/2 justify-center pointer-events-none sm:top-1.5">',
    '      <div className={cn("absolute top-[calc(6px+env(safe-area-inset-top,0px))] left-1/2 z-30 flex -translate-x-1/2 justify-center pointer-events-none sm:top-1.5", chromeClass)}>',
)
s = s.replace(
    '      <div className="absolute top-[calc(6px+env(safe-area-inset-top,0px))] left-3 z-30 sm:left-1.5 sm:top-1.5">',
    '      <div className={cn("absolute top-[calc(6px+env(safe-area-inset-top,0px))] left-3 z-30 sm:left-1.5 sm:top-1.5", chromeClass)}>',
)
s = s.replace(
    '<div className={isMobile ? "bg-bg-panel/70 backdrop-blur-xl rounded-[26px] p-2 border border-border-primary/40" : "contents"}>',
    '<div className={cn(isMobile ? "bg-bg-panel/70 backdrop-blur-xl rounded-[26px] p-2 border border-border-primary/40" : "contents", isMobile && chromeClass)}>',
)

assert s != o, "no changes"
p.write_text(s, encoding="utf-8")
print("fullscreen patched")
