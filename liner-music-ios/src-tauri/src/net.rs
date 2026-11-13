//! Connectivity diagnostics.
//!
//! The desktop build shells out to Node's `dns`, `tls` and `net` and to the OS
//! `traceroute`. Under iOS there is no process to spawn, so each stage is
//! re-implemented with tokio + rustls primitives. The **JSON shape is
//! identical** to `electron/netdiag.ts` / `electron/netprobe/types.ts`, because
//! the renderer's diagnostics sheet reads those fields directly.
//!
//! `trace` is always `null`: iOS does not expose traceroute and faking it would
//! put misleading data in an issue report.

use std::time::{Duration, Instant};

use serde::Serialize;
use tokio::net::{lookup_host, TcpStream};

use crate::error::AppResult;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DnsCheck {
    pub host: String,
    pub ok: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub addresses: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resolvers: Option<Vec<String>>,
    pub ms: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TcpCheck {
    pub host: String,
    pub port: u16,
    pub ok: bool,
    pub ms: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TlsCheck {
    pub host: String,
    pub ok: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub protocol: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub authorized: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub subject: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub issuer: Option<String>,
    pub ms: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ControlCheck {
    pub name: String,
    pub ok: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<u16>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
    pub ms: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TraceResult {
    pub host: String,
    pub ok: bool,
    pub hops: Vec<String>,
    pub ms: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MainNetResult {
    pub dns: Vec<DnsCheck>,
    pub tcp: Vec<TcpCheck>,
    pub tls: Vec<TlsCheck>,
    pub controls: Vec<ControlCheck>,
    pub trace: Option<TraceResult>,
    pub platform: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProbeResult {
    pub internet: bool,
    pub youtube: bool,
    pub youtube_failure: Option<String>,
    pub country: Option<String>,
}

/// Run every stage for up to `hosts.len()` hosts.
pub async fn diagnose(http: &reqwest::Client, hosts: Vec<String>) -> AppResult<MainNetResult> {
    let mut dns = Vec::with_capacity(hosts.len());
    let mut tcp = Vec::with_capacity(hosts.len());
    let mut tls = Vec::with_capacity(hosts.len());

    for host in hosts {
        // ---- DNS ----
        let started = Instant::now();
        let resolved = tokio::time::timeout(
            Duration::from_secs(5),
            lookup_host((host.as_str(), 443u16)),
        )
        .await;

        let mut ip_literal = None;
        match resolved {
            Ok(Ok(addrs)) => {
                let mut addresses: Vec<String> = addrs.map(|a| a.ip().to_string()).collect();
                addresses.sort();
                addresses.dedup();
                ip_literal = addresses.first().cloned();
                dns.push(DnsCheck {
                    host: host.clone(),
                    ok: !addresses.is_empty(),
                    addresses: Some(addresses),
                    resolvers: None,
                    ms: started.elapsed().as_millis() as u64,
                    error: None,
                });
            }
            Ok(Err(err)) => dns.push(DnsCheck {
                host: host.clone(),
                ok: false,
                addresses: None,
                resolvers: None,
                ms: started.elapsed().as_millis() as u64,
                error: Some(err.to_string()),
            }),
            Err(_) => dns.push(DnsCheck {
                host: host.clone(),
                ok: false,
                addresses: None,
                resolvers: None,
                ms: started.elapsed().as_millis() as u64,
                error: Some("timeout".into()),
            }),
        }

        // ---- TCP :443 ----
        let started = Instant::now();
        let tcp_target = ip_literal.clone().unwrap_or_else(|| host.clone());
        let connected = tokio::time::timeout(
            Duration::from_secs(6),
            TcpStream::connect((tcp_target.as_str(), 443u16)),
        )
        .await;
        let tcp_ok = matches!(&connected, Ok(Ok(_)));
        tcp.push(TcpCheck {
            host: host.clone(),
            port: 443,
            ok: tcp_ok,
            ms: started.elapsed().as_millis() as u64,
            error: match &connected {
                Ok(Ok(_)) => None,
                Ok(Err(err)) => Some(err.to_string()),
                Err(_) => Some("timeout".into()),
            },
        });

        // ---- TLS ----
        // A real handshake through rustls: if the certificate chain does not
        // validate, the request fails and we report `authorized: false`. This is
        // the check that catches captive portals and MITM proxies.
        let started = Instant::now();
        let response = http
            .head(format!("https://{host}/"))
            .timeout(Duration::from_secs(8))
            .send()
            .await;

        match response {
            Ok(resp) => tls.push(TlsCheck {
                host: host.clone(),
                ok: true,
                protocol: Some(format!("{:?}", resp.version())),
                authorized: Some(true),
                subject: None,
                issuer: None,
                ms: started.elapsed().as_millis() as u64,
                error: None,
            }),
            Err(err) => {
                let message = err.to_string();
                let cert_issue = message.contains("certificate") || message.contains("tls");
                tls.push(TlsCheck {
                    host: host.clone(),
                    ok: false,
                    protocol: None,
                    authorized: Some(!cert_issue),
                    subject: None,
                    issuer: None,
                    ms: started.elapsed().as_millis() as u64,
                    error: Some(message),
                });
            }
        }
    }

    // ---- control endpoints ----
    let mut controls = Vec::new();
    for (name, url) in [
        ("liner-api", "https://api.tryliner.fun/v1/health"),
        ("covers", "https://covers.tryliner.fun/"),
        ("youtube", "https://www.youtube.com/generate_204"),
    ] {
        let started = Instant::now();
        let result = http
            .get(url)
            .timeout(Duration::from_secs(8))
            .send()
            .await;
        controls.push(match result {
            Ok(response) => ControlCheck {
                name: name.into(),
                ok: response.status().is_success() || response.status().is_redirection(),
                status: Some(response.status().as_u16()),
                detail: None,
                ms: started.elapsed().as_millis() as u64,
                error: None,
            },
            Err(err) => ControlCheck {
                name: name.into(),
                ok: false,
                status: None,
                detail: None,
                ms: started.elapsed().as_millis() as u64,
                error: Some(err.to_string()),
            },
        });
    }

    Ok(MainNetResult {
        dns,
        tcp,
        tls,
        controls,
        trace: None,
        platform: "ios".into(),
    })
}

/// Lightweight "is the network sane" probe used by the connectivity banner.
pub async fn probe(http: &reqwest::Client) -> ProbeResult {
    const YOUTUBE_PROBE: &str = "https://www.youtube.com/generate_204";
    const LINER_PROBE: &str = "https://api.tryliner.fun/v1/health";
    const GEO_PROBE: &str = "https://www.cloudflare.com/cdn-cgi/trace";

    let liner = http
        .get(LINER_PROBE)
        .timeout(Duration::from_secs(8))
        .send()
        .await
        .map(|r| r.status().is_success())
        .unwrap_or(false);

    let youtube_response = http
        .get(YOUTUBE_PROBE)
        .timeout(Duration::from_secs(8))
        .send()
        .await;

    let (youtube, youtube_failure) = match youtube_response {
        Ok(response) if response.status().is_success() || response.status().is_redirection() => {
            (true, None)
        }
        Ok(response) => (false, Some(format!("blocked:{}", response.status().as_u16()))),
        Err(err) if err.is_timeout() => (false, Some("timeout".into())),
        Err(_) => (false, Some("blocked".into())),
    };

    // `loc=SA` line of the Cloudflare trace; best-effort, no failure impact.
    let country = match http
        .get(GEO_PROBE)
        .timeout(Duration::from_secs(5))
        .send()
        .await
    {
        Ok(response) => match response.text().await {
            Ok(body) => body
                .lines()
                .find_map(|line| line.strip_prefix("loc=").map(str::to_string))
                .filter(|code| code.len() == 2),
            Err(_) => None,
        },
        Err(_) => None,
    };

    ProbeResult {
        internet: liner,
        youtube,
        youtube_failure,
        country,
    }
}
