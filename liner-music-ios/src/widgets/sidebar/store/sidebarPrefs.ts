import { create } from "zustand";

/**
 * Which sidebar entries are shown.
 *
 * Persisted to localStorage because it is a layout preference, not app state —
 * it should survive a reinstall of the shell (and the WebView's storage is the
 * only durable place the renderer owns on iOS).
 *
 * `history` is the back/forward pair that moved out of the window header.
 */
export interface SidebarVisibility {
  history: boolean;
  home: boolean;
  search: boolean;
  library: boolean;
  settings: boolean;
  librarySection: boolean;
}

interface SidebarPrefsStore {
  visible: SidebarVisibility;
  setVisible: (id: keyof SidebarVisibility, value: boolean) => void;
  reset: () => void;
}

const STORAGE_KEY = "liner_sidebar_visibility";

const DEFAULTS: SidebarVisibility = {
  history: true,
  home: true,
  search: true,
  library: true,
  settings: true,
  librarySection: true,
};

function read(): SidebarVisibility {
  if (typeof localStorage === "undefined") return DEFAULTS;
  try {
    const raw = localStorage.getItem(STORAGE_KEY);
    if (!raw) return DEFAULTS;
    const parsed = JSON.parse(raw) as Partial<SidebarVisibility>;
    return { ...DEFAULTS, ...parsed };
  } catch {
    return DEFAULTS;
  }
}

function write(value: SidebarVisibility): void {
  try {
    localStorage.setItem(STORAGE_KEY, JSON.stringify(value));
  } catch {
    /* storage full or blocked: the preference simply does not persist */
  }
}

export const useSidebarPrefs = create<SidebarPrefsStore>((set, get) => ({
  visible: read(),
  setVisible: (id, value) => {
    const next = { ...get().visible, [id]: value };
    write(next);
    set({ visible: next });
  },
  reset: () => {
    write(DEFAULTS);
    set({ visible: DEFAULTS });
  },
}));
