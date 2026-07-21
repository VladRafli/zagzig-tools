import { useTranslation } from "react-i18next";

import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { setLanguage, type SupportedLanguage } from "@/i18n";

const LANGUAGE_NAMES: Record<SupportedLanguage, string> = {
  en: "English",
  id: "Bahasa Indonesia",
};

export function LanguageSwitcher() {
  const { i18n } = useTranslation();

  return (
    <Select
      value={i18n.language}
      onValueChange={(value) => setLanguage(value as SupportedLanguage)}
    >
      <SelectTrigger size="sm" className="w-full">
        <SelectValue />
      </SelectTrigger>
      <SelectContent>
        {Object.entries(LANGUAGE_NAMES).map(([code, name]) => (
          <SelectItem key={code} value={code}>
            {name}
          </SelectItem>
        ))}
      </SelectContent>
    </Select>
  );
}
