import { ThemeProvider as NextThemesProvider, useTheme as useNextTheme } from "next-themes";
import {
  createContext,
  useContext,
  useMemo,
  useState,
  type CSSProperties,
  type ReactNode,
} from "react";

const themeDefinitions = [
  { id: "ocean", label: "Ocean", hue: 252 },
  { id: "sage", label: "Sage", hue: 145 },
  { id: "sand", label: "Sand", hue: 85 },
  { id: "clay", label: "Clay", hue: 35 },
  { id: "lilac", label: "Lilac", hue: 300 },
] as const;

export const themes = themeDefinitions.map(({ hue, ...theme }) => ({
  ...theme,
  light: {
    accent: `oklch(0.5 0.03 ${hue} / 7%)`,
    border: `oklch(0.89 0.006 ${hue})`,
    icon: `oklch(0.5 0.045 ${hue})`,
    sidebar: `oklch(0.96 0.008 ${hue})`,
  },
  dark: {
    accent: `oklch(0.75 0.03 ${hue} / 9%)`,
    border: `oklch(0.32 0.008 ${hue})`,
    icon: `oklch(0.7 0.04 ${hue})`,
    sidebar: `oklch(0.24 0.01 ${hue})`,
  },
}));

type ThemeId = (typeof themes)[number]["id"];
type ThemeAppearance = "light" | "dark";

type ThemeContextValue = {
  appearance: ThemeAppearance;
  setAppearance: (appearance: ThemeAppearance) => void;
  setThemeId: (themeId: ThemeId) => void;
  themeId: ThemeId;
};

const ThemeContext = createContext<ThemeContextValue | null>(null);

export function ThemeProvider({ children }: { children: ReactNode }) {
  return (
    <NextThemesProvider attribute="class" defaultTheme="system" storageKey="theme">
      <ThemeStateProvider>{children}</ThemeStateProvider>
    </NextThemesProvider>
  );
}

function ThemeStateProvider({ children }: { children: ReactNode }) {
  const { resolvedTheme, setTheme } = useNextTheme();
  const [themeId, setThemeId] = useState<ThemeId>("ocean");
  const appearance: ThemeAppearance = resolvedTheme === "dark" ? "dark" : "light";
  const theme = themes.find(({ id }) => id === themeId) ?? themes[0];
  const colors = theme[appearance];
  const value = useMemo(
    () => ({
      appearance,
      setAppearance: (nextAppearance: ThemeAppearance) => setTheme(nextAppearance),
      setThemeId,
      themeId,
    }),
    [appearance, setTheme, themeId],
  );

  return (
    <ThemeContext.Provider value={value}>
      <div
        className="contents"
        data-theme={themeId}
        style={
          {
            "--sidebar-accent": colors.accent,
            "--sidebar-border": colors.border,
            "--sidebar": colors.sidebar,
            "--theme-icon": colors.icon,
          } as CSSProperties
        }
      >
        {children}
      </div>
    </ThemeContext.Provider>
  );
}

export function useTheme() {
  const context = useContext(ThemeContext);
  if (!context) throw new Error("useTheme must be used within ThemeProvider");
  return context;
}
