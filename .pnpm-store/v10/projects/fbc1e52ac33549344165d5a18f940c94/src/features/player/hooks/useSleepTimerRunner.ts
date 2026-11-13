import { useEffect } from "react";

import { playerEngine } from "../engine/playerEngine";
import { useSleepTimerStore } from "../store/sleepTimerStore";

/** Fade window before the timer stops playback. */
const FADE_MS = 30_000;

/**
 * Drives the sleep timer: fades the last 30 s, pauses at the deadline, and
 * hands the volume back afterwards. Mounted once for the whole app.
 */
export function useSleepTimerRunner(): void {
  const endsAt = useSleepTimerStore((state) => state.endsAt);
  const clear = useSleepTimerStore((state) => state.clear);

  useEffect(() => {
    if (endsAt === null) return;

    // Remember the level to restore after the fade.
    const baseVolume = playerEngine.getSnapshot().volume || 1;
    let faded = false;

    const tick = () => {
      const remaining = endsAt - Date.now();

      if (remaining <= 0) {
        playerEngine.pause();
        playerEngine.setVolume(baseVolume);
        clear();
        return;
      }

      if (remaining <= FADE_MS) {
        faded = true;
        const gain = Math.max(0, remaining / FADE_MS);
        playerEngine.setVolume(baseVolume * gain);
      } else if (faded) {
        faded = false;
        playerEngine.setVolume(baseVolume);
      }
    };

    tick();
    const id = window.setInterval(tick, 1000);
    return () => {
      window.clearInterval(id);
      if (faded) playerEngine.setVolume(baseVolume);
    };
  }, [endsAt, clear]);
}
