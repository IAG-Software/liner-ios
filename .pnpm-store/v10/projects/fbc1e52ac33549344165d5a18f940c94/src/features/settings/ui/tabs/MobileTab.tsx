import { SettingRow, SettingSection } from "../controls";
import { useMobileSettings } from "../../store/mobileSettings";
import { useTranslation } from "@/languages";

interface MobileTabProps {
  searchQuery: string;
}

/**
 * Phone-only switches. Everything here maps to something the device actually
 * does differently — none of it applies to the desktop build.
 */
export default function MobileTab({ searchQuery }: MobileTabProps) {
  const { t } = useTranslation();
  const settings = useMobileSettings();

  const row = (
    key: "autoHideChrome" | "extraEdgeInset",
    titleKey: string,
    descKey: string,
  ) => (
    <SettingRow
      searchQuery={searchQuery}
      title={t(titleKey)}
      description={t(descKey)}
      control={
        <button
          type="button"
          role="switch"
          aria-checked={settings[key]}
          onClick={() => settings.set(key, !settings[key])}
          className={`relative h-[30px] w-[50px] cursor-pointer rounded-full border-0 transition-colors duration-200 ${
            settings[key] ? "bg-accent-primary" : "bg-border-alpha-33"
          }`}
        >
          <span
            className={`absolute top-[3px] h-[24px] w-[24px] rounded-full bg-white shadow-sm transition-all duration-200 ${
              settings[key] ? "left-[23px]" : "left-[3px]"
            }`}
          />
        </button>
      }
    />
  );

  return (
    <div className="flex flex-col gap-[6px]">
      <SettingSection label={t("settings.mobile.section_player")}>
        {row(
          "autoHideChrome",
          "settings.mobile.auto_hide.title",
          "settings.mobile.auto_hide.description",
        )}
      </SettingSection>

      <SettingSection label={t("settings.mobile.section_layout")}>
        {row(
          "extraEdgeInset",
          "settings.mobile.edge_inset.title",
          "settings.mobile.edge_inset.description",
        )}
      </SettingSection>
    </div>
  );
}
