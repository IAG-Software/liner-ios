import { LeftLine, RightLine } from "@mingcute/react";

import { useHistoryNavigation } from "@/widgets/header/lib/useHistoryNavigation";
import { useTranslation } from "@/languages";
import { cn } from "@/shared/lib";
import { useSidebarPrefs } from "../store/sidebarPrefs";

const NAV_BUTTON_CLASS =
  "flex h-[28px] w-[28px] items-center justify-center rounded-[8px] border-0 bg-transparent p-0 cursor-pointer select-none text-text-primary opacity-55 transition-[background-color,color,opacity] duration-150 hover:opacity-100 hover:bg-border-alpha-14 active:scale-95 disabled:!opacity-20 disabled:cursor-default disabled:pointer-events-none";

/**
 * Back / forward, living in the sidebar rail.
 *
 * They used to sit in the window header next to the traffic lights; with the
 * window chrome gone the rail is the only chrome left, and history is the one
 * thing that has nowhere else to go on a touch layout (no ⌘[ , no mouse buttons).
 */
export function SidebarHistoryNav() {
  const { t } = useTranslation();
  const { canGoBack, canGoForward, goBack, goForward } = useHistoryNavigation();
  const showHistory = useSidebarPrefs((state) => state.visible.history);

  if (!showHistory) return null;

  return (
    <div className="flex flex-col items-center gap-[2px]">
      <button
        type="button"
        onClick={goBack}
        disabled={!canGoBack}
        aria-label={t("common.back")}
        className={NAV_BUTTON_CLASS}
      >
        <LeftLine size={17} />
      </button>
      <button
        type="button"
        onClick={goForward}
        disabled={!canGoForward}
        aria-label={t("common.forward")}
        className={cn(NAV_BUTTON_CLASS)}
      >
        <RightLine size={17} />
      </button>
    </div>
  );
}
