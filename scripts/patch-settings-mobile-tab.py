from pathlib import Path

p = Path(r"D:\ios2\liner-music-ios\src\features\settings\ui\SettingsModal.tsx")
s = p.read_text(encoding="utf-8")
original = s

s = s.replace('  | "Storage"\n  | "About";', '  | "Storage"\n  | "Mobile"\n  | "About";')
s = s.replace(
    '  Storage: Database,\n  About: InfoCircle,',
    '  Storage: Database,\n  Mobile: Widget5,\n  About: InfoCircle,',
)
s = s.replace(
    '{ labelKey: "interface", ids: ["Appearance"] },',
    '{ labelKey: "interface", ids: ["Appearance", "Mobile"] },',
)
s = s.replace(
    '    Storage: t("settings.tabs.storage"),\n    About: t("settings.tabs.about"),',
    '    Storage: t("settings.tabs.storage"),\n    Mobile: t("settings.tabs.mobile") || "Mobile",\n    About: t("settings.tabs.about"),',
)
s = s.replace(
    '    Storage: t("settings.storage.description"),\n    About: t("settings.about.tagline"),',
    '    Storage: t("settings.storage.description"),\n'
    '    Mobile:\n'
    '      t("settings.mobile.description") ||\n'
    '      "Phone-only behaviour: idle chrome, the island pill and edge padding.",\n'
    '    About: t("settings.about.tagline"),',
)
s = s.replace(
    '      Storage: "settings.tabs.storage",\n      About: "settings.tabs.about",',
    '      Storage: "settings.tabs.storage",\n      Mobile: "settings.tabs.mobile",\n      About: "settings.tabs.about",',
)
s = s.replace(
    '      Storage: "settings.storage.description",',
    '      Storage: "settings.storage.description",\n      Mobile: "settings.mobile.description",',
)
s = s.replace('  YoutubeTab,\n} from "./tabs";', '  YoutubeTab,\n  MobileTab,\n} from "./tabs";')
s = s.replace(
    '              {activeTab === "About" && <AboutTab searchQuery={searchQuery} />}',
    '              {activeTab === "Mobile" && (\n'
    '                <MobileTab searchQuery={searchQuery} />\n'
    '              )}\n'
    '              {activeTab === "About" && <AboutTab searchQuery={searchQuery} />}',
)

assert s != original, "no replacements applied"
p.write_text(s, encoding="utf-8")
print("settings patched")
