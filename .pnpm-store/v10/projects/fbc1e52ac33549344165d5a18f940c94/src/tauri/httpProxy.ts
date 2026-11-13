/**
 * Native fetch proxy.
 *
 * Two WKWebView limitations make direct API calls impossible:
 *
 *  1. `User-Agent` is a *forbidden header name* for `fetch`, so the renderer
 *     cannot present itself as the desktop client — and the API's Cloudflare
 *     rule only admits `Electron/` UAs (403 for everything else, measured).
 *  2. The document origin is `tauri://localhost`, which no server's
 *     `Access-Control-Allow-Origin` is written for.
 *
 * So Liner hosts are replayed through the `http_request` Rust command, which
 * sets the right UA and is not subject to CORS. Everything else (media CDNs,
 * cover art) stays on WebKit, because that keeps the genuine Safari TLS
 * fingerprint and streams without buffering through the IPC bridge.
 *
 * If the native call fails, the original `fetch` is used so behaviour degrades
 * to exactly what it was before this shim existed.
 */

import { invoke } from "@tauri-apps/api/core";

interface ProxyResponse {
  status: number;
  statusText: string;
  headers: [string, string][];
  body: string;
}

/** Liner's own API surface; everything else is not proxied. */
const PROXIED_HOST = /(^|\.)tryliner\.fun$/i;

const FORBIDDEN_FOR_RESPONSE = new Set([
  "content-encoding",
  "content-length",
  "transfer-encoding",
  "connection",
]);

function base64FromBytes(bytes: Uint8Array): string {
  let binary = "";
  const chunk = 0x8000;
  for (let i = 0; i < bytes.length; i += chunk) {
    binary += String.fromCharCode(
      ...bytes.subarray(i, Math.min(i + chunk, bytes.length)),
    );
  }
  return btoa(binary);
}

function bytesFromBase64(value: string): Uint8Array {
  if (!value) return new Uint8Array(0);
  const binary = atob(value);
  const out = new Uint8Array(binary.length);
  for (let i = 0; i < binary.length; i += 1) out[i] = binary.charCodeAt(i);
  return out;
}

async function proxyFetch(
  input: RequestInfo | URL,
  init?: RequestInit,
): Promise<Response> {
  const request =
    input instanceof Request && !init
      ? input.clone()
      : new Request(input as RequestInfo, init);

  const hasBody = request.method !== "GET" && request.method !== "HEAD";
  const buffer = hasBody ? await request.arrayBuffer() : null;

  const headers: [string, string][] = [];
  request.headers.forEach((value, key) => {
    headers.push([key, value]);
  });

  const result = await invoke<ProxyResponse>("http_request", {
    request: {
      method: request.method,
      url: request.url,
      headers,
      body: buffer && buffer.byteLength > 0
        ? base64FromBytes(new Uint8Array(buffer))
        : null,
    },
  });

  const responseHeaders = new Headers();
  for (const [key, value] of result.headers) {
    if (FORBIDDEN_FOR_RESPONSE.has(key.toLowerCase())) continue;
    try {
      responseHeaders.append(key, value);
    } catch {
      /* a header WebKit refuses is not worth failing the request over */
    }
  }

  const bytes = bytesFromBase64(result.body);
  const isNullBody =
    result.status === 204 || result.status === 205 || result.status === 304;

  // `bytes.buffer` is exact: `bytesFromBase64` always allocates a fresh view.
  return new Response(isNullBody ? null : (bytes.buffer as ArrayBuffer), {
    status: result.status,
    statusText: result.statusText,
    headers: responseHeaders,
  });
}

let installed = false;

/** Replace `window.fetch` with the proxying version. Idempotent. */
export function installHttpProxy(): void {
  if (installed || typeof window === "undefined") return;
  installed = true;

  const original = window.fetch.bind(window);

  window.fetch = async (
    input: RequestInfo | URL,
    init?: RequestInit,
  ): Promise<Response> => {
    let hostname = "";
    try {
      const url = input instanceof Request ? input.url : String(input);
      hostname = new URL(url, window.location.href).hostname;
    } catch {
      return original(input as RequestInfo, init);
    }

    if (PROXIED_HOST.test(hostname)) {
      try {
        return await proxyFetch(input, init);
      } catch (error) {
        console.warn("[liner] native proxy failed, retrying in WebKit", error);
        return original(input as RequestInfo, init);
      }
    }

    try {
      return await original(input as RequestInfo, init);
    } catch (error) {
      // A cross-origin block surfaces as a TypeError here. One retry through
      // the native client is strictly better than failing outright.
      try {
        return await proxyFetch(input, init);
      } catch {
        throw error;
      }
    }
  };
}
