import { ColorsIcon } from "@hugeicons/core-free-icons";
import { useTheme } from "next-themes";

import {
  NativeSelect,
  NativeSelectOptGroup,
  NativeSelectOption,
} from "@/components/ui/native-select";

export function ThemePicker() {
  const { theme, setTheme } = useTheme();

  return (
    <NativeSelect
      aria-label="Appearance"
      className="mr-1.5 shrink-0 [-webkit-app-region:no-drag]"
      icon={ColorsIcon}
      iconClassName="text-muted-foreground"
      onChange={(event) => {
        const value = event.currentTarget.value;
        if (value === "system" || value === "light" || value === "dark") setTheme(value);
      }}
      size="sm"
      value={theme ?? "system"}
      variant="icon"
    >
      <NativeSelectOptGroup label="Appearance">
        <NativeSelectOption value="system">System</NativeSelectOption>
        <NativeSelectOption value="light">Light</NativeSelectOption>
        <NativeSelectOption value="dark">Dark</NativeSelectOption>
      </NativeSelectOptGroup>
    </NativeSelect>
  );
}
