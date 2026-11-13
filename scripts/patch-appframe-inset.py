from pathlib import Path

p = Path(r"D:\ios2\liner-music-ios\src\features\navigation\ui\AppFrame.tsx")
s = p.read_text(encoding="utf-8")
o = s

s = s.replace(
    'import { useIsMobile } from "@/shared/hooks/useIsMobile";',
    'import { useIsMobile } from "@/shared/hooks/useIsMobile";\n'
    'import { useMobileSettings } from "@/features/settings/store/mobileSettings";',
)
s = s.replace(
    "  const isMobile = useIsMobile();",
    "  const isMobile = useIsMobile();\n"
    "  // iPhone 15/16/17 curve their corners hard enough to clip a rail that sits\n"
    "  // 6px from the edge, so phones get a wider gutter by default.\n"
    "  const extraEdgeInset = useMobileSettings((s) => s.extraEdgeInset);",
)
s = s.replace(
    "      className={`app-shell-enter relative flex flex-col h-full w-full overflow-hidden gap-1.5 p-1.5 pt-[max(6px,env(safe-area-inset-top,0px))] ${",
    "      className={`app-shell-enter relative flex flex-col h-full w-full overflow-hidden gap-1.5 p-1.5 pt-[max(6px,env(safe-area-inset-top,0px))] ${\n"
    '        extraEdgeInset ? "max-sm:px-3 max-sm:pb-3" : ""\n'
    "      } ${",
)

assert s != o, "no changes"
p.write_text(s, encoding="utf-8")
print("appframe patched")
