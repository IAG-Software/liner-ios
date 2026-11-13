import { useEffect } from "react";
import { useNavigate } from "react-router-dom";

import { useModalStore } from "@/features/library";

/**
 * `liner://` deep links.
 *
 * This is what makes the app scriptable *today*, without App Intents: Siri
 * shortcuts, the Shortcuts "Open URL" action, the Today widget and the iMessage
 * card all hand us a URL, and this maps it onto a route.
 *
 * Supported: `liner://home`, `liner://library`, `liner://library/saved`,
 * `liner://library/local`, `liner://library/cached`, `liner://search`.
 */
export function DeepLinkRouter() {
  const navigate = useNavigate();
  const openSearch = useModalStore((state) => state.openSearch);

  useEffect(() => {
    const api = window.linerElectron;
    if (!api?.onOpenUrl) return;

    return api.onOpenUrl((raw) => {
      let parsed: URL;
      try {
        parsed = new URL(raw);
      } catch {
        return;
      }
      if (parsed.protocol !== "liner:") return;

      // `liner://library` parses host=library; `liner:///library` parses path.
      const target = (parsed.hostname || parsed.pathname.replace(/^\/+/, ""))
        .toLowerCase()
        .replace(/\/+$/, "");

      switch (target) {
        case "":
        case "home":
          navigate("/");
          break;
        case "library":
          navigate("/library");
          break;
        case "library/saved":
        case "saved":
          navigate("/library/saved");
          break;
        case "library/local":
        case "local":
          navigate("/library/local");
          break;
        case "library/cached":
        case "cached":
          navigate("/library/cached");
          break;
        case "search":
          openSearch();
          break;
        default:
          navigate("/");
      }
    });
  }, [navigate, openSearch]);

  return null;
}

export default DeepLinkRouter;
