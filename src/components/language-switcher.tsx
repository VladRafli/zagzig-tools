import { useTranslation } from "react-i18next";

import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { setLanguage, useLanguages } from "@/i18n";

export function LanguageSwitcher() {
  const { i18n } = useTranslation();
  const languages = useLanguages();

  return (
    <Select value={i18n.language} onValueChange={(value) => setLanguage(value as string)}>
      <SelectTrigger size="sm" className="w-full">
        <SelectValue>
          {(value: string) => languages.find((l) => l.code === value)?.name ?? value}
        </SelectValue>
      </SelectTrigger>
      <SelectContent>
        {languages.map((language) => (
          <SelectItem key={language.code} value={language.code}>
            {language.name}
          </SelectItem>
        ))}
      </SelectContent>
    </Select>
  );
}
