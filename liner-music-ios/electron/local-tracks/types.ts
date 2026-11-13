// See `electron/preload.ts` — type-only bridge onto `src/tauri/api.ts`.
export type {
  LocalImportResult,
  LocalLibraryUsage,
  LocalTrackRecord,
} from "../../src/tauri/api";

/** Mirrors upstream `electron/local-tracks/types.ts`. */
export const LOCAL_AUDIO_EXTENSIONS = [
  "mp3",
  "flac",
  "m4a",
  "aac",
  "ogg",
  "opus",
  "wav",
  "webm",
] as const;
