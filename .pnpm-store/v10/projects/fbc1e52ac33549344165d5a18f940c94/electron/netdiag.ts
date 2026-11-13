// See `electron/preload.ts` — type-only bridge onto `src/tauri/api.ts`.
//
// The diagnostic *shape* is preserved: the renderer's "connection problems"
// sheet reads `dns`/`tcp`/`tls`/`controls`/`trace` directly, and the Rust
// implementation in `src-tauri/src/net.rs` returns exactly these fields.
export type { MainNetResult } from "../src/tauri/api";

export const MAIN_NET_HOST_RE =
  /^(?=.{1,253}$)[a-z0-9]([a-z0-9.-]*[a-z0-9])?$/i;
export const MAIN_NET_MAX_HOSTS = 5;
export const MAIN_NET_TRACE_MAX_HOPS = 12;

export function maskIp(ip: string): string {
  if (/^\d+\.\d+\.\d+\.\d+$/.test(ip))
    return ip.split(".").slice(0, 2).concat("x", "x").join(".");
  if (ip.includes(":")) return `${ip.split(":").slice(0, 2).join(":")}:x:x`;
  return "x";
}
