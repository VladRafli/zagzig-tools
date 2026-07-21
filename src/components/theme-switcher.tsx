import { useTheme } from "next-themes";
import { useTranslation } from "react-i18next";

import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";

type ThemeOption = "system" | "light" | "dark";

const THEME_OPTIONS: ThemeOption[] = ["system", "light", "dark"];

export function ThemeSwitcher() {
  const { t } = useTranslation();
  const { theme, setTheme } = useTheme();

  return (
    <Select
      value={(theme as ThemeOption) ?? "system"}
      onValueChange={(value) => setTheme((value as ThemeOption) ?? "system")}
    >
      <SelectTrigger size="sm" className="w-full" aria-label={t("app.theme")}>
        <SelectValue />
      </SelectTrigger>
      <SelectContent>
        {THEME_OPTIONS.map((option) => (
          <SelectItem key={option} value={option}>
            {t(`theme.${option}`)}
          </SelectItem>
        ))}
      </SelectContent>
    </Select>
  );
}
