import i18next from "i18next";
import { initReactI18next } from "react-i18next";

import en from "./locales/en";
import id from "./locales/id";

export const LANGUAGE_STORAGE_KEY = "zagzig:language";
export const SUPPORTED_LANGUAGES = ["en", "id"] as const;
export type SupportedLanguage = (typeof SUPPORTED_LANGUAGES)[number];

function initialLanguage(): SupportedLanguage {
  try {
    const stored = localStorage.getItem(LANGUAGE_STORAGE_KEY);
    if (stored && (SUPPORTED_LANGUAGES as readonly string[]).includes(stored)) {
      return stored as SupportedLanguage;
    }
  } catch {
    // localStorage unavailable — fall through to default
  }
  return "en";
}

i18next.use(initReactI18next).init({
  resources: {
    en: { translation: en },
    id: { translation: id },
  },
  lng: initialLanguage(),
  fallbackLng: "en",
  interpolation: { escapeValue: false },
});

export function setLanguage(language: SupportedLanguage) {
  void i18next.changeLanguage(language);
  try {
    localStorage.setItem(LANGUAGE_STORAGE_KEY, language);
  } catch {
    // best-effort persistence
  }
}

export default i18next;
