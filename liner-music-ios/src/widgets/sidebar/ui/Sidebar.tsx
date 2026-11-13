import { memo, useCallback, useEffect, useRef, useState } from "react";
import { Link } from "react-router-dom";
import logo from "@/assets/logo.svg";
import { SidebarLibrary } from "./SidebarLibrary";
import { SidebarNavItems } from "./SidebarNavItems";
import { SidebarSettingsButton } from "./SidebarSettingsButton";
import { SidebarHistoryNav } from "./SidebarHistoryNav";
import SidebarCustomizeDialog from "./SidebarCustomizeDialog";
import { useSidebarPrefs } from "../store/sidebarPrefs";

export interface SidebarProps {
  searchOpen: boolean;
  onSearchToggle: () => void;
}

/** Hold this long to open the sidebar editor. */
const LONG_PRESS_MS = 550;

function Sidebar({ searchOpen, onSearchToggle }: SidebarProps) {
  const [customizeOpen, setCustomizeOpen] = useState(false);
  const timerRef = useRef<number | null>(null);
  const firedRef = useRef(false);
  const showLibrarySection = useSidebarPrefs(
    (state) => state.visible.librarySection,
  );

  const clearTimer = useCallback(() => {
    if (timerRef.current !== null) {
      window.clearTimeout(timerRef.current);
      timerRef.current = null;
    }
  }, []);

  useEffect(() => clearTimer, [clearTimer]);

  const handlePointerDown = useCallback(() => {
    firedRef.current = false;
    clearTimer();
    timerRef.current = window.setTimeout(() => {
      firedRef.current = true;
      setCustomizeOpen(true);
    }, LONG_PRESS_MS);
  }, [clearTimer]);

  // Swallow the click that would otherwise navigate after a long press.
  const handleClickCapture = useCallback((event: React.MouseEvent) => {
    if (firedRef.current) {
      event.preventDefault();
      event.stopPropagation();
      firedRef.current = false;
    }
  }, []);

  return (
    <div className="h-full w-[52px] flex flex-col items-center gap-[6px]">
      {/* one panel for nav and library; hugs its content and scrolls the
          library once it reaches the window bottom */}
      <div
        className="w-full min-h-0 flex flex-col rounded-item bg-bg-glass backdrop-blur-[20px]"
        onPointerDown={handlePointerDown}
        onPointerUp={clearTimer}
        onPointerLeave={clearTimer}
        onPointerCancel={clearTimer}
        onClickCapture={handleClickCapture}
        onContextMenu={(event) => event.preventDefault()}
      >
        <nav className="shrink-0 flex flex-col items-center gap-[14px] py-[14px]">
          <Link
            to="/"
            title="Liner"
            className="flex items-center justify-center cursor-pointer active:scale-95 transition-transform"
          >
            <img
              src={logo}
              alt="Liner"
              width={30}
              height={23}
              className="h-[23px] w-auto object-contain theme-logo invert dark:invert-0 select-none"
              draggable={false}
            />
          </Link>

          <SidebarHistoryNav />

          <SidebarNavItems
            searchOpen={searchOpen}
            onSearchToggle={onSearchToggle}
          />
          <SidebarSettingsButton />
        </nav>

        {showLibrarySection ? <SidebarLibrary /> : null}
      </div>

      <SidebarCustomizeDialog
        open={customizeOpen}
        onOpenChange={setCustomizeOpen}
      />
    </div>
  );
}

export default memo(Sidebar);
