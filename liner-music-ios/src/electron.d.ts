// The desktop bridge type lived in `electron/preload.ts`, which does not exist
// in the iOS port. `src/tauri/api.ts` declares the equivalent global
// (`window.linerElectron`), so this file only pulls that declaration in.
import "./tauri/api";

export {};
