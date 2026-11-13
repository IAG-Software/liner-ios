"""
Which part of the User-Agent does the Cloudflare rule match?

/v1/me answers 401 UNAUTHORIZED for any request that reaches the origin, and
Cloudflare's block page answers 403. So 401 = passed the WAF, 403 = blocked.
"""

import ssl
import urllib.error
import urllib.request

BASE = "https://api.tryliner.fun"
PATH = "/v1/me"

CANDIDATES = {
    "safari-ios (control)":
        "Mozilla/5.0 (iPhone; CPU iPhone OS 18_0 like Mac OS X) AppleWebKit/605.1.15 "
        "(KHTML, like Gecko) Version/18.0 Mobile/15E148 Safari/604.1",
    "mac + liner/0.2.13":
        "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 "
        "(KHTML, like Gecko) liner/0.2.13 Chrome/130.0.0.0 Electron/33.2.0 Safari/537.36",
    "windows + liner/0.2.13":
        "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 "
        "(KHTML, like Gecko) liner/0.2.13 Chrome/130.0.0.0 Electron/33.2.0 Safari/537.36",
    "mac + Electron only (no liner)":
        "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 "
        "(KHTML, like Gecko) Chrome/130.0.0.0 Electron/33.2.0 Safari/537.36",
    "desktop Chrome (no Electron)":
        "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 "
        "(KHTML, like Gecko) Chrome/130.0.0.0 Safari/537.36",
    "bare liner/0.2.13":
        "liner/0.2.13",
}

for label, ua in CANDIDATES.items():
    req = urllib.request.Request(BASE + PATH)
    req.add_header("User-Agent", ua)
    req.add_header("Accept", "application/json")
    try:
        with urllib.request.urlopen(req, timeout=20, context=ssl.create_default_context()) as r:
            print(f"{label:32s} {r.status}  (passed WAF)")
    except urllib.error.HTTPError as e:
        payload = e.read(160).decode("utf-8", "replace")
        blocked = "Attention Required" in payload
        verdict = "BLOCKED by WAF" if blocked else "passed WAF"
        print(f"{label:32s} {e.code}  {verdict}")
    except Exception as e:  # noqa: BLE001
        print(f"{label:32s} ERR  {type(e).__name__}: {e}")
