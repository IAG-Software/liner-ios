import Dialog from "@/shared/ui/Dialog";
import ToggleSwitch from "@/shared/ui/ToggleSwitch";
import { useTranslation } from "@/languages";
import { useSidebarPrefs, type SidebarVisibility } from "../store/sidebarPrefs";

interface SidebarCustomizeDialogProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
}

const ROWS: Array<{ id: keyof SidebarVisibility; labelKey: string }> = [
  { id: "history", labelKey: "sidebar.customize.history" },
  { id: "home", labelKey: "common.sidebar.home" },
  { id: "search", labelKey: "common.sidebar.search" },
  { id: "library", labelKey: "common.sidebar.library" },
  { id: "librarySection", labelKey: "sidebar.customize.library_section" },
  { id: "settings", labelKey: "common.sidebar.settings" },
];

/**
 * Opened by holding the sidebar. Toggles are applied live and persisted, so the
 * rail can be pared down to just what the user uses.
 */
export default function SidebarCustomizeDialog({
  open,
  onOpenChange,
}: SidebarCustomizeDialogProps) {
  const { t } = useTranslation();
  const visible = useSidebarPrefs((state) => state.visible);
  const setVisible = useSidebarPrefs((state) => state.setVisible);
  const reset = useSidebarPrefs((state) => state.reset);

  return (
    <Dialog open={open} onOpenChange={onOpenChange} maxWidth={340}>
      <div className="flex flex-col gap-[2px] px-[18px] pb-[16px]">
        <div className="pb-[10px] pt-[2px]">
          <h2 className="m-0 text-[15px] font-semibold text-text-primary">
            {t("sidebar.customize.title")}
          </h2>
          <p className="m-0 mt-[3px] text-[12px] leading-snug text-text-tertiary">
            {t("sidebar.customize.subtitle")}
          </p>
        </div>

        {ROWS.map((row) => (
          <label
            key={row.id}
            className="flex cursor-pointer items-center justify-between gap-3 rounded-[10px] px-[2px] py-[9px]"
          >
            <span className="text-[13.5px] text-text-primary">
              {t(row.labelKey)}
            </span>
            <ToggleSwitch
              checked={visible[row.id]}
              onChange={(next) => setVisible(row.id, next)}
            />
          </label>
        ))}

        <button
          type="button"
          onClick={reset}
          className="mt-[10px] h-[34px] cursor-pointer rounded-[10px] border border-border-primary/60 bg-bg-elevated text-[13px] font-medium text-text-secondary transition-colors active:scale-[0.99]"
        >
          {t("sidebar.customize.reset")}
        </button>
      </div>
    </Dialog>
  );
}
