// See `electron/preload.ts` — type-only bridge onto `src/tauri/api.ts`.
export type {
  SavedLibraryUsage,
  SavedLyricsState,
  SavedTrackInput,
  SavedTrackMeta,
  SavedTrackRecord,
  SavedTracksPartUsage,
} from "../../src/tauri/api";
