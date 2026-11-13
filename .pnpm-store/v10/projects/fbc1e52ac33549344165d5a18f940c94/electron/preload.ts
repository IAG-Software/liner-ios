/**
 * Compatibility shim.
 *
 * Upstream renderer modules do `import type { … } from "../../electron/preload"`.
 * In this port the bridge lives in `src/tauri/api.ts`, but the renderer sources
 * are kept byte-identical to upstream so the two trees stay mergeable. This
 * directory maps the old module paths onto the new contract — type-only, no
 * runtime code is emitted.
 *
 * If upstream ever stops importing from `electron/`, delete this folder.
 */
export * from "../src/tauri/api";
export type { LinerNativeApi as LinerElectronApi } from "../src/tauri/api";
