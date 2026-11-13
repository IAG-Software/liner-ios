//! Shared HTTP client + the User-Agent the backend expects.
//!
//! ## Why the UA is load-bearing
//!
//! `api.tryliner.fun` sits behind a Cloudflare WAF rule that admits the desktop
//! client and rejects everything else. Measured against `/v1/me` (which answers
//! `401 UNAUTHORIZED` as soon as a request reaches the origin, versus a `403`
//! block page at the edge):
//!
//! | User-Agent                                          | Result |
//! |-----------------------------------------------------|--------|
//! | iOS Safari                                          | `403`  |
//! | desktop Chrome (no `Electron/`)                     | `403`  |
//! | `liner/0.2.13` alone                                | `403`  |
//! | Electron UA (with or without the app name)          | `401`  |
//!
//! So the rule keys on `Electron/` — i.e. it admits the desktop app. A
//! WKWebView cannot produce that: `User-Agent` is a forbidden header name for
//! `fetch`, so the renderer physically cannot send it. Every request therefore
//! has to leave through Rust, which is what `commands::http_cmds` is for.
//!
//! This is a compatibility shim around a server-side rule we do not control. If
//! the WAF is ever tightened to fingerprint the TLS stack instead of the UA,
//! this stops working and the API calls fail with a 403 body we surface verbatim.

/// Desktop-client User-Agent (Electron's default shape for this app).
pub const DESKTOP_UA: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) \
AppleWebKit/537.36 (KHTML, like Gecko) liner/0.2.13 Chrome/130.0.0.0 \
Electron/33.2.0 Safari/537.36";

/// Hosts that must go through the native proxy.
#[allow(dead_code)] // documentation of the rule the renderer shim implements
pub fn is_liner_host(host: &str) -> bool {
    let host = host.to_ascii_lowercase();
    host == "tryliner.fun" || host.ends_with(".tryliner.fun")
}

pub fn build_client() -> reqwest::Result<reqwest::Client> {
    reqwest::Client::builder()
        .user_agent(DESKTOP_UA)
        .timeout(std::time::Duration::from_secs(60))
        .connect_timeout(std::time::Duration::from_secs(15))
        .pool_idle_timeout(std::time::Duration::from_secs(90))
        .pool_max_idle_per_host(8)
        .build()
}
