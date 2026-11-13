import { useEffect, useState } from "react";

/**
 * True on phone-sized viewports.
 *
 * The renderer was written for a desktop window; this hook is the single source
 * of truth for "we are on a phone", so layout branches do not each invent their
 * own media query. 640px matches Tailwind's `sm` breakpoint, and the CSS classes
 * used alongside it (`sm:`) agree by construction.
 *
 * `window.matchMedia` is evaluated in a lazy initialiser so the very first paint
 * already has the right value — no desktop layout flash on a phone.
 */
export function useIsMobile(): boolean {
  const [isMobile, setIsMobile] = useState(() => {
    if (typeof window === "undefined") return false;
    return window.matchMedia("(max-width: 639px)").matches;
  });

  useEffect(() => {
    const query = window.matchMedia("(max-width: 639px)");
    const onChange = (event: MediaQueryListEvent) => setIsMobile(event.matches);
    setIsMobile(query.matches);
    query.addEventListener("change", onChange);
    return () => query.removeEventListener("change", onChange);
  }, []);

  return isMobile;
}

/** True for iOS/iPadOS, where the safe-area insets are non-zero. */
export function isIOS(): boolean {
  if (typeof navigator === "undefined") return false;
  const ua = navigator.userAgent;
  return /iPhone|iPad|iPod/.test(ua);
}
