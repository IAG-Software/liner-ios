"""
Probe api.tryliner.fun exactly the way the desktop app does.

If the WAF is letting the real client through, the discriminator is one of:
  * the x-liner-* HMAC headers (signer/src/crypto.rs), or
  * the User-Agent.

This reproduces the HMAC-SHA256 canonical string
    METHOD \n PATH \n TIMESTAMP \n NONCE \n SHA256_HEX(BODY)
with the obfuscated key deobfuscated exactly as the Rust crate does, then
issues the request with and without the headers so the two outcomes can be
compared.
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

# --- constants copied from signer/src/crypto.rs -----------------------------
OBF_API_SEED = bytes([
    0x55, 0xc3, 0x5c, 0xaa, 0x3f, 0x3c, 0x32, 0x6d, 0xaa, 0x63, 0x0f, 0x90,
    0x39, 0x5d, 0x66, 0x36,
])
OBF_API_KEY = bytes([
    0x24, 0x99, 0x56, 0x2b, 0x80, 0x20, 0xc1, 0xb7, 0xf6, 0x7d, 0x74, 0x91,
    0xe4, 0xc3, 0x7f, 0x67, 0x1a, 0x0e, 0x8a, 0xf3, 0x7f, 0x50, 0xaf, 0x2e,
    0x05, 0xd5, 0xa7, 0xb0, 0xbe, 0x6e, 0x31, 0x11, 0x40, 0xf3, 0x42, 0xad,
    0x31, 0xf6, 0xcb, 0xea, 0xb5, 0xba, 0xdd, 0xd0, 0x8e, 0x56, 0x55, 0x8c,
    0xb4, 0xe5, 0x1d, 0x82, 0x69, 0x0c, 0x3f, 0xb2, 0x30, 0x82, 0xa7, 0xf4,
    0x9b, 0xba, 0x9b, 0x70,
])
API_KEY_CHECKSUM = bytes([0xe2, 0x09, 0x02, 0x55, 0x83, 0xee, 0xf0, 0x07])

KEY_LEN = 64


def deobfuscate(seed: bytes, obf: bytes) -> bytes:
    out = b""
    i = 0
    while len(out) < KEY_LEN:
        out += hashlib.sha256(seed + bytes([i])).digest()
        i = (i + 1) & 0xFF
    ks = out[:KEY_LEN]
    key = bytes(a ^ b for a, b in zip(obf, ks))
    assert hashlib.sha256(key).digest()[:8] == API_KEY_CHECKSUM, "checksum mismatch"
    return key


def sign(path: str, method: str = "GET", body: str = ""):
    key = deobfuscate(OBF_API_SEED, OBF_API_KEY)
    ts = int(time.time())
    nonce = secrets.token_hex(8)
    body_hash = hashlib.sha256(body.encode()).hexdigest()
    canonical = f"{method.upper()}\n{path}\n{ts}\n{nonce}\n{body_hash}"
    sig = base64.urlsafe_b64encode(
        hmac.new(key, canonical.encode(), hashlib.sha256).digest()
    ).decode().rstrip("=")
    return sig, ts, nonce


SAFARI_UA = (
    "Mozilla/5.0 (iPhone; CPU iPhone OS 18_0 like Mac OS X) "
    "AppleWebKit/605.1.15 (KHTML, like Gecko) Version/18.0 Mobile/15E148 Safari/604.1"
)


def probe(label: str, url: str, path: str, headers: dict, method="GET", body=None):
    req = urllib.request.Request(url, method=method, data=body)
    for k, v in headers.items():
        req.add_header(k, v)
    ctx = ssl.create_default_context()
    try:
        with urllib.request.urlopen(req, timeout=25, context=ctx) as resp:
            payload = resp.read(400).decode("utf-8", "replace")
            print(f"{label:38s} {resp.status}  {payload[:160]!r}")
    except urllib.error.HTTPError as err:
        payload = err.read(200).decode("utf-8", "replace")
        marker = "CLOUDFLARE-BLOCK" if "Attention Required" in payload or "blocked" in payload else ""
        print(f"{label:38s} {err.code}  {marker} {payload[:80]!r}")
    except Exception as err:  # noqa: BLE001
        print(f"{label:38s} ERR  {err}")


BASE = "https://api.tryliner.fun"
ART = "/v1/health"

sig, ts, nonce = sign(ART)
print("signing key reconstructed OK; sample signature:", sig[:16], "...\n")

probe("plain GET (no extra headers)", BASE + ART, ART, {"User-Agent": SAFARI_UA})
probe(
    "signed GET",
    BASE + ART,
    ART,
    {
        "User-Agent": SAFARI_UA,
        "Accept": "application/json",
        "x-liner-signature": sig,
        "x-liner-timestamp": str(ts),
        "x-liner-nonce": nonce,
    },
)

login_body = json.dumps({"email": "probe@example.com", "password": "x" * 10})
lsig, lts, lnonce = sign("/v1/auth/login", "POST", login_body)
probe(
    "signed POST /v1/auth/login",
    BASE + "/v1/auth/login",
    "/v1/auth/login",
    {
        "User-Agent": SAFARI_UA,
        "Content-Type": "application/json",
        "Accept": "application/json",
        "x-liner-signature": lsig,
        "x-liner-timestamp": str(lts),
        "x-liner-nonce": lnonce,
    },
    method="POST",
    body=login_body.encode(),
)
