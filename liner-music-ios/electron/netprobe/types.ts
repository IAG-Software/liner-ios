// See `electron/preload.ts` — type-only bridge onto `src/tauri/api.ts`.
export type { NetProbeResult } from "../../src/tauri/api";

export type YoutubeFailure = "timeout" | "blocked";
