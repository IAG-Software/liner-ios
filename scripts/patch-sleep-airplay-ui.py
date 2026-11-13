from pathlib import Path

# ---- App.tsx: mount the runner ---------------------------------------------
p = Path(r"D:\ios2\liner-music-ios\src\app\App.tsx")
s = p.read_text(encoding="utf-8")
if "useSleepTimerRunner" not in s:
    s = s.replace(
        'import { useThemeCustomizationSync } from "@/features/settings";',
        'import { useThemeCustomizationSync } from "@/features/settings";\n'
        'import { useSleepTimerRunner } from "@/features/player/hooks/useSleepTimerRunner";',
    )
    s = s.replace(
        "function DiscordSync() {",
        "function SleepTimerRunner() {\n  useSleepTimerRunner();\n  return null;\n}\n\nfunction DiscordSync() {",
    )
    s = s.replace("        <DiscordSync />", "        <DiscordSync />\n        <SleepTimerRunner />")
p.write_text(s, encoding="utf-8")

# ---- FullscreenPlayer: AirPlay + sleep timer cluster ------------------------
p = Path(r"D:\ios2\liner-music-ios\src\features\player\ui\FullscreenPlayer.tsx")
s = p.read_text(encoding="utf-8")

if "AirplayLine" not in s:
    s = s.replace(
        'import { Notch } from "@/shared/ui";',
        'import { AirplayLine, MoonLine } from "@mingcute/react";\n'
        'import { Notch } from "@/shared/ui";\n'
        'import { useSleepTimerStore } from "../store/sleepTimerStore";',
    )

anchor = "  const isMobile = useIsMobile();"
if "useSleepTimerStore(" not in s:
    s = s.replace(
        anchor,
        anchor
        + "\n  const sleepEndsAt = useSleepTimerStore((state) => state.endsAt);\n"
        + "  const startSleepTimer = useSleepTimerStore((state) => state.start);\n"
        + "  const clearSleepTimer = useSleepTimerStore((state) => state.clear);\n"
        + "  const [timerOpen, setTimerOpen] = useState(false);\n"
        + "  // re-render once a minute so the remaining label stays honest\n"
        + "  useEffect(() => {\n"
        + "    if (sleepEndsAt === null) return;\n"
        + "    const id = window.setInterval(() => setTimerOpen((v) => v), 30_000);\n"
        + "    return () => window.clearInterval(id);\n"
        + "  }, [sleepEndsAt]);\n"
        + "  const sleepLabel =\n"
        + "    sleepEndsAt === null\n"
        + "      ? null\n"
        + "      : `${Math.max(1, Math.round((sleepEndsAt - Date.now()) / 60_000))}m`;",
    )

cluster = '''
      {/* outputs + sleep timer, where the window controls used to be */}
      <div
        className={cn(
          "absolute top-[calc(6px+env(safe-area-inset-top,0px))] right-3 z-30 flex items-center gap-[6px] sm:right-1.5 sm:top-1.5",
          chromeClass,
        )}
      >
        <button
          type="button"
          aria-label="AirPlay"
          onClick={() => void window.linerElectron?.showRoutePicker?.()}
          className="flex h-[30px] w-[30px] items-center justify-center rounded-full border-0 bg-white/10 text-white/85 backdrop-blur-xl active:scale-95"
        >
          <AirplayLine size={16} />
        </button>

        <div className="relative">
          <button
            type="button"
            aria-label="Sleep timer"
            onClick={() => setTimerOpen((v) => !v)}
            className={cn(
              "flex h-[30px] items-center gap-[4px] rounded-full border-0 px-[9px] text-white/85 backdrop-blur-xl active:scale-95",
              sleepLabel ? "bg-white/20" : "bg-white/10",
            )}
          >
            <MoonLine size={16} />
            {sleepLabel ? (
              <span className="text-[11px] font-medium">{sleepLabel}</span>
            ) : null}
          </button>

          {timerOpen ? (
            <div className="absolute right-0 top-[36px] w-[142px] overflow-hidden rounded-[14px] border border-border-primary/60 bg-bg-panel/95 p-1 backdrop-blur-xl">
              {[15, 30, 45, 60].map((minutes) => (
                <button
                  key={minutes}
                  type="button"
                  onClick={() => {
                    startSleepTimer(minutes);
                    setTimerOpen(false);
                  }}
                  className="flex h-[34px] w-full cursor-pointer items-center justify-between rounded-[9px] border-0 bg-transparent px-[10px] text-[13px] text-text-primary active:bg-white/10"
                >
                  <span>{minutes} min</span>
                  {sleepLabel && Math.round((sleepEndsAt! - Date.now()) / 60_000) === minutes ? (
                    <span className="text-[11px] text-text-tertiary">on</span>
                  ) : null}
                </button>
              ))}
              <button
                type="button"
                onClick={() => {
                  clearSleepTimer();
                  setTimerOpen(false);
                }}
                className="flex h-[34px] w-full cursor-pointer items-center rounded-[9px] border-0 bg-transparent px-[10px] text-[13px] text-text-tertiary active:bg-white/10"
              >
                Off
              </button>
            </div>
          ) : null}
        </div>
      </div>

'''

marker = '      <div className={cn("absolute top-[calc(6px+env(safe-area-inset-top,0px))] left-3 z-30 sm:left-1.5 sm:top-1.5", chromeClass)}>'
if "Sleep timer" not in s:
    s = s.replace(marker, cluster + marker, 1)

p.write_text(s, encoding="utf-8")
print("sleep timer + airplay ui wired")
