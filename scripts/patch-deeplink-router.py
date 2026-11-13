from pathlib import Path

p = Path(r"D:\ios2\liner-music-ios\src\app\App.tsx")
s = p.read_text(encoding="utf-8")
if "DeepLinkRouter" not in s:
    s = s.replace(
        'import DeeplinkHandler from "./DeeplinkHandler";',
        'import DeeplinkHandler from "./DeeplinkHandler";\nimport { DeepLinkRouter } from "./DeepLinkRouter";',
    )
    s = s.replace(
        "                  <DeeplinkHandler />",
        "                  <DeeplinkHandler />\n                  <DeepLinkRouter />",
    )
p.write_text(s, encoding="utf-8")
print("deep link router mounted")
