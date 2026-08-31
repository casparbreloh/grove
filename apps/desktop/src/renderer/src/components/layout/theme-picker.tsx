import { ColorsIcon } from "@hugeicons/core-free-icons";

import { themes, useTheme } from "@/components/theme-provider";
import {
  NativeSelect,
  NativeSelectOptGroup,
  NativeSelectOption,
} from "@/components/ui/native-select";

const lightAppearanceValue = "appearance:light";
const darkAppearanceValue = "appearance:dark";
const appearanceMenuValue = "";

export function ThemePicker() {
  const { appearance, setAppearance, setThemeId, themeId } = useTheme();

  return (
    <NativeSelect
      aria-label="Appearance and sidebar color"
      className="mr-1.5 shrink-0 [-webkit-app-region:no-drag]"
      icon={ColorsIcon}
      iconClassName="text-[var(--theme-icon)]"
      onChange={(event) => {
        const value = event.currentTarget.value;
        const theme = themes.find(({ id }) => id === value);
        if (theme) setThemeId(theme.id);
        if (value === lightAppearanceValue) setAppearance("light");
        if (value === darkAppearanceValue) setAppearance("dark");
        event.currentTarget.value = appearanceMenuValue;
      }}
      size="sm"
      value={appearanceMenuValue}
      variant="icon"
    >
      <NativeSelectOption disabled hidden value={appearanceMenuValue}>
        Appearance
      </NativeSelectOption>
      <NativeSelectOptGroup label="Color">
        {themes.map(({ id, label }) => (
          <NativeSelectOption key={id} value={id}>
            {label}
            {themeId === id ? " ✓" : ""}
          </NativeSelectOption>
        ))}
      </NativeSelectOptGroup>
      <NativeSelectOptGroup label="Appearance">
        <NativeSelectOption value={lightAppearanceValue}>
          Light{appearance === "light" ? " ✓" : ""}
        </NativeSelectOption>
        <NativeSelectOption value={darkAppearanceValue}>
          Dark{appearance === "dark" ? " ✓" : ""}
        </NativeSelectOption>
      </NativeSelectOptGroup>
    </NativeSelect>
  );
}
