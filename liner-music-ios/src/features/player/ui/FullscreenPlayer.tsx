import { useCallback, useEffect, useRef, useState } from "react";
import { useNavigate } from "react-router-dom";
import { usePlayerStore } from "../store/playerStore";
import { useGlassBlur, useKawarpEnabled } from "../store/visualModeStore";
import { cn } from "@/shared/lib";
import { useIsMobile } from "@/shared/hooks/useIsMobile";
import {
  CHROME_IDLE_MS,
  useMobileSettings,
} from "@/features/settings/store/mobileSettings";
import { AirplayLine, MoonLine } from "@mingcute/react";
import { Notch } from "@/shared/ui";
import { useSleepTimerStore } from "../store/sleepTimerStore";
import { FullscreenTopNotch } from "./FullscreenTopNotch";
import { LyricsNotchStatus } from "./LyricsNotchStatus";
import { FullscreenFrameGlass } from "./FullscreenFrameGlass";
import { useLyricsStore, LyricsProviderIsland } from "@/features/lyrics";
import { AnimatePresence, motion } from "framer-motion";
import { PlayerUiElements } from "./PlayerUiElements";
import { DownLine } from "@mingcute/react";
import { preloadCoverArt, useCoverSrc } from "@/features/covers";
import KawarpWrapper from "./KawarpWrapper";
import AmbientLayer from "./AmbientLayer";
import { BetaVisualsWarning } from "./BetaVisualsWarning";
import { CanvasLyricsView } from "./CanvasLyricsView";
import { useTranslation } from "@/languages";
import { useSettledLyricsLayout } from "../hooks/useSettledLyricsLayout";

const KAWARP_FADE_IN_MS = 600;

export interface FullscreenPlayerProps {
  effectsReady?: boolean;
  onClose: () => void;
}

export function FullscreenPlayer({
  effectsReady = true,
  onClose,
}: FullscreenPlayerProps) {
  const { t } = useTranslation();
  const { autoHideChrome } = useMobileSettings();
  const currentTrack = usePlayerStore((s) => s.currentTrack);
  const nextCoverUrl = usePlayerStore(
    (s) => s.queue[s.currentIndex + 1]?.coverUrl,
  );
  const coverUrl = currentTrack?.coverUrl;
  const kawarpSrc = useCoverSrc(coverUrl);

  useEffect(() => {
    if (!currentTrack) {
      onClose();
    }
  }, [currentTrack, onClose]);

  // --- idle chrome ---------------------------------------------------------
  // After three seconds without a touch the transport and the top chrome fade
  // away, leaving the lyrics alone on screen. Any tap brings them back.
  const [chromeVisible, setChromeVisible] = useState(true);
  const idleTimerRef = useRef<number | null>(null);

  const bumpChrome = useCallback(() => {
    setChromeVisible(true);
    if (idleTimerRef.current !== null) window.clearTimeout(idleTimerRef.current);
    if (!autoHideChrome) return;
    idleTimerRef.current = window.setTimeout(() => {
      setChromeVisible(false);
      idleTimerRef.current = null;
    }, CHROME_IDLE_MS);
  }, [autoHideChrome]);

  useEffect(() => {
    bumpChrome();
    return () => {
      if (idleTimerRef.current !== null) window.clearTimeout(idleTimerRef.current);
    };
  }, [bumpChrome]);

  useEffect(() => {
    if (!autoHideChrome) setChromeVisible(true);
    else bumpChrome();
  }, [autoHideChrome, bumpChrome]);

  const chromeClass = cn(
    "transition-opacity duration-300 ease-out",
    chromeVisible ? "opacity-100" : "opacity-0 pointer-events-none",
  );

  const {
    braccatoLyrics,
    lyricsLoading,
    lyricsError,
    plainLyrics,
    instrumental,
    availableProviders,
    currentLyricsTrackId,
  } = useLyricsStore();

  const onError = useCallback((err: Error) => {
    console.error("kawarp load error:", err);
  }, []);

  const kawarpEnabled = useKawarpEnabled();
  const glassBlur = useGlassBlur();
  const [kawarpShown, setKawarpShown] = useState(false);
  const [kawarpWasEnabled, setKawarpWasEnabled] = useState(kawarpEnabled);
  if (kawarpWasEnabled !== kawarpEnabled) {
    setKawarpWasEnabled(kawarpEnabled);
    setKawarpShown(false);
  }
  const onKawarpLoad = useCallback(() => {
    setTimeout(() => setKawarpShown(true), KAWARP_FADE_IN_MS);
  }, []);

  useEffect(() => {
    void preloadCoverArt(coverUrl);
    void preloadCoverArt(nextCoverUrl);
  }, [coverUrl, nextCoverUrl]);

  const currentTrackId = currentTrack?.id;
  const isMatchingTrack = Boolean(
    currentTrackId && currentLyricsTrackId === currentTrackId,
  );
  const hasLyrics = isMatchingTrack && braccatoLyrics.length > 0;
  const showProviderIsland =
    isMatchingTrack && availableProviders.length > (hasLyrics ? 0 : 1);
  const isInstrumental =
    isMatchingTrack &&
    !lyricsLoading &&
    !hasLyrics &&
    !lyricsError &&
    (instrumental || plainLyrics === "[INSTRUMENTAL]");
  const isLyricsLoading = !hasLyrics && lyricsLoading;
  const isLyricsMissing = isMatchingTrack && !hasLyrics && !lyricsLoading;
  const isMobile = useIsMobile();
  const sleepEndsAt = useSleepTimerStore((state) => state.endsAt);
  const startSleepTimer = useSleepTimerStore((state) => state.start);
  const clearSleepTimer = useSleepTimerStore((state) => state.clear);
  const [timerOpen, setTimerOpen] = useState(false);
  // re-render once a minute so the remaining label stays honest
  useEffect(() => {
    if (sleepEndsAt === null) return;
    const id = window.setInterval(() => setTimerOpen((v) => v), 30_000);
    return () => window.clearInterval(id);
  }, [sleepEndsAt]);
  const sleepLabel =
    sleepEndsAt === null
      ? null
      : `${Math.max(1, Math.round((sleepEndsAt - Date.now()) / 60_000))}m`;
  const isCompactLayout = useSettledLyricsLayout({
    isMatchingTrack,
    lyricsLoading,
    hasLyrics,
  });

  return (
    <div
      data-theme="dark"
      className="dark relative h-full w-full overflow-hidden bg-black text-white"
      onPointerDown={bumpChrome}
    >
      <div className="absolute inset-0 overflow-hidden">
        <div
          aria-hidden="true"
          className="absolute inset-0"
          style={{ backgroundColor: "var(--color-bg-primary)" }}
        />
        {(kawarpSrc || coverUrl) && (!kawarpShown || !kawarpEnabled) && (
          <img
            src={kawarpSrc || coverUrl}
            alt=""
            aria-hidden="true"
            draggable={false}
            className="absolute inset-0 h-full w-full object-cover pointer-events-none"
            style={{
              filter: "blur(64px) saturate(1.3)",
              transform: "scale(1.3)",
            }}
          />
        )}
        {effectsReady && kawarpEnabled ? (
          <KawarpWrapper
            src={kawarpSrc}
            onLoad={onKawarpLoad}
            onError={onError}
          />
        ) : null}
        <AmbientLayer trackId={currentTrackId} />
      </div>

      <div className="absolute inset-0 z-[1] bg-black/40" />

      <FullscreenFrameGlass />

      <div className="absolute inset-0 z-10 flex flex-col px-3 pt-[env(safe-area-inset-top,0px)] pb-[env(safe-area-inset-bottom,0px)] sm:inset-1.5 sm:flex-row sm:p-0">
        <div
          className={cn(
            "flex flex-col",
            isMobile
              ? "order-2 absolute inset-x-0 bottom-0 z-20 pb-2"
              : "flex-1 items-center justify-center gap-5",
          )}
        >
          {/* Phone: the controls ride on a liquid-glass pane at the bottom, so
              the lyrics get the whole upper screen without being clipped. */}
          <div className={cn(isMobile ? "bg-bg-panel/70 backdrop-blur-xl rounded-[26px] p-2 border border-border-primary/40" : "contents", isMobile && chromeClass)}>
            <PlayerUiElements
              onClose={onClose}
              layout={isMobile || isCompactLayout ? "horizontal" : "vertical"}
            />
          </div>
        </div>

        <div
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
              ? "order-1 flex-1 py-2"
              : cn(
                  "h-full",
                  isCompactLayout
                    ? "grow-0 basis-0 px-0"
                    : "grow basis-0 pl-2 pr-6",
                ),
          )}
        >
          <div
            className="flex-1 min-h-0 relative w-full h-full"
            style={{
              display: hasLyrics ? "flex" : "none",
            }}
          >
            <AnimatePresence initial={false}>
              {hasLyrics && (
                <motion.div
                  key={`lyrics-${currentTrackId}`}
                  initial={{ opacity: 0 }}
                  animate={{ opacity: 1 }}
                  exit={{ opacity: 0, pointerEvents: "none" }}
                  transition={{ duration: 0.22, ease: "easeInOut" }}
                  className="absolute inset-0 flex flex-col"
                >
                  <CanvasLyricsView lyrics={braccatoLyrics} />
                </motion.div>
              )}
            </AnimatePresence>
          </div>
        </div>
      </div>

      <div className={cn("absolute top-[calc(6px+env(safe-area-inset-top,0px))] left-1/2 z-30 flex -translate-x-1/2 justify-center pointer-events-none sm:top-1.5", chromeClass)}>
        <FullscreenTopNotch>
          <BetaVisualsWarning docked={false} />
          {showProviderIsland && <LyricsProviderIsland />}
          {isLyricsLoading && <LyricsNotchStatus kind="loading" />}
          {isLyricsMissing && (
            <LyricsNotchStatus
              kind={isInstrumental ? "instrumental" : "missing"}
            />
          )}
        </FullscreenTopNotch>
      </div>


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

      <div className={cn("absolute top-[calc(6px+env(safe-area-inset-top,0px))] left-3 z-30 sm:left-1.5 sm:top-1.5", chromeClass)}>
        <Notch align="start" tone="glass" blur={glassBlur}>
          <button
            type="button"
            aria-label={t("player.close_fullscreen")}
            onClick={onClose}
            data-no-window-drag
            className="inline-flex h-[26px] w-[26px] cursor-pointer items-center justify-center rounded-control border-none bg-transparent text-white/70 transition-colors select-none hover:bg-white/10 hover:text-white active:scale-95"
          >
            <DownLine size={16} />
          </button>
        </Notch>
      </div>
    </div>
  );
}

export default function PlayerPage() {
  const navigate = useNavigate();
  return <FullscreenPlayer onClose={() => navigate(-1)} />;
}
