import { create } from "zustand";

/**
 * Sleep timer.
 *
 * Stores an absolute deadline rather than a countdown so it survives the app
 * being backgrounded or the WebView being throttled — a `setInterval` counter
 * would drift (or stop) the moment iOS suspends the page.
 */
interface SleepTimerStore {
  /** Epoch ms when playback should stop, or null when inactive. */
  endsAt: number | null;
  /** Chosen preset, for showing it in the UI. */
  minutes: number | null;
  start: (minutes: number) => void;
  clear: () => void;
}

export const useSleepTimerStore = create<SleepTimerStore>((set) => ({
  endsAt: null,
  minutes: null,
  start: (minutes) =>
    set({ endsAt: Date.now() + minutes * 60_000, minutes }),
  clear: () => set({ endsAt: null, minutes: null }),
}));

export function sleepRemainingMs(): number | null {
  const { endsAt } = useSleepTimerStore.getState();
  if (endsAt === null) return null;
  return Math.max(0, endsAt - Date.now());
}
