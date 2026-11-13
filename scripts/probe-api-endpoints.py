"""
The WAF discriminates on User-Agent: iOS/Safari UAs get Cloudflare's 403 block,
an Electron-style UA reaches the origin (404 instead of 403).

This probes real endpoints with the desktop UA + valid HMAC to find out what the
API actually answers, which tells us what the native proxy must send.
"""

import base64
import hashlib
import hmac
import json
import secrets
import ssl
import time
import urllib.error
import urllib.request

OBF_API_SEED = bytes([0x55,0xc3,0x5c,0xaa,0x3f,0x3c,0x32,0x6d,0xaa,0x63,0x0f,0x90,0x39,0x5d,0x66,0x36])
OBF_API_KEY = bytes([
    0x24,0x99,0x56,0x2b,0x80,0x20,0xc1,0xb7,0xf6,0x7d,0x74,0x91,0xe4,0xc3,0x7f,0x67,
    0x1a,0x0e,0x8a,0xf3,0x7f,0x50,0xaf,0x2e,0x05,0xd5,0xa7,0xb0,0xbe,0x6e,0x31,0x11,
    0x40,0xf3,0x42,0xad,0x31,0xf6,0xcb,0xea,0xb5,0xba,0xdd,0xd0,0x8e,0x56,0x55,0x8c,
    0xb4,0xe5,0x1d,0x82,0x69,0x0c,0x3f,0xb2,0x30,0x82,0xa7,0xf4,0x9b,0xba,0x9b,0x70,
])
API_KEY_CHECKSUM = bytes([0xe2,0x09,0x02,0x55,0x83,0xee,0xf0,0x07])

ELECTRON_UA = (
    "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 "
    "(KHTML, like Gecko) liner/0.2.12 Chrome/130.0.0.0 Electron/33.2.0 Safari/537.36"
)


def key() -> bytes:
    out, i = b"", 0
    while len(out) < 64:
        out += hashlib.sha256(OBF_API_SEED + bytes([i])).digest()
        i += 1
    k = bytes(a ^ b for a, b in zip(OBF_API_KEY, out[:64]))
    assert hashlib.sha256(k).digest()[:8] == API_KEY_CHECKSUM
    return k


def sign(path: str, method: str = "GET", body: str = ""):
    ts = int(time.time())
    nonce = secrets.token_hex(8)
    canonical = f"{method.upper()}\n{path}\n{ts}\n{nonce}\n{hashlib.sha256(body.encode()).hexdigest()}"
    sig = base64.urlsafe_b64encode(
        hmac.new(key(), canonical.encode(), hashlib.sha256).digest()
    ).decode().rstrip("=")
    return sig, ts, nonce


def call(label, path, method="GET", body=None):
    sig, ts, nonce = sign(path, method, body or "")
    data = body.encode() if body else None
    req = urllib.request.Request("https://api.tryliner.fun" + path, method=method, data=data)
    headers = {
        "User-Agent": ELECTRON_UA,
        "Accept": "application/json",
        "x-liner-signature": sig,
        "x-liner-timestamp": str(ts),
        "x-liner-nonce": nonce,
    }
    if body:
        headers["Content-Type"] = "application/json"
    for k, v in headers.items():
        req.add_header(k, v)
    try:
        with urllib.request.urlopen(req, timeout=25, context=ssl.create_default_context()) as r:
            print(f"{label:44s} {r.status}  {r.read(240).decode('utf-8','replace')[:200]!r}")
    except urllib.error.HTTPError as e:
        payload = e.read(240).decode("utf-8", "replace")
        tag = " [CLOUDFLARE]" if "Attention Required" in payload else ""
        print(f"{label:44s} {e.code}{tag}  {payload[:150]!r}")
    except Exception as e:  # noqa: BLE001
        print(f"{label:44s} ERR  {e}")


call("GET /", "/")
call("GET /v1/health", "/v1/health")
call("GET /v1/tracks/search?q=test", "/v1/tracks/search?q=test")
call("GET /v1/me (no token)", "/v1/me")
call("POST /v1/auth/login (bad creds)", "/v1/auth/login", "POST",
     json.dumps({"email": "probe@example.com", "password": "not-a-real-password"}))
