import { create } from "zustand";

/** Phone-only presentation preferences. */
export interface MobileSettings {
  /** Hide transport + chrome after 3 s without a touch, leaving pure lyrics. */
  autoHideChrome: boolean;
  /** Wider gutter around the shell, for phones with aggressive corner radii. */
  extraEdgeInset: boolean;
}

interface MobileSettingsStore extends MobileSettings {
  set: <K extends keyof MobileSettings>(key: K, value: MobileSettings[K]) => void;
  reset: () => void;
}

const STORAGE_KEY = "liner_mobile_settings";

const DEFAULTS: MobileSettings = {
  autoHideChrome: true,
  extraEdgeInset: true,
};

function read(): MobileSettings {
  if (typeof localStorage === "undefined") return DEFAULTS;
  try {
    const raw = localStorage.getItem(STORAGE_KEY);
    if (!raw) return DEFAULTS;
    return { ...DEFAULTS, ...(JSON.parse(raw) as Partial<MobileSettings>) };
  } catch {
    return DEFAULTS;
  }
}

function write(value: MobileSettings): void {
  try {
    localStorage.setItem(STORAGE_KEY, JSON.stringify(value));
  } catch {
    /* non-persistent is better than throwing on every toggle */
  }
}

export const useMobileSettings = create<MobileSettingsStore>((set, get) => ({
  ...read(),
  set: (key, value) => {
    const next: MobileSettings = {
      autoHideChrome: get().autoHideChrome,
      extraEdgeInset: get().extraEdgeInset,
      [key]: value,
    };
    write(next);
    set({ [key]: value } as Pick<MobileSettingsStore, typeof key>);
  },
  reset: () => {
    write(DEFAULTS);
    set(DEFAULTS);
  },
}));

/** Idle delay before the fullscreen chrome collapses. */
export const CHROME_IDLE_MS = 3000;
