import { useSyncExternalStore } from "react";
import { invoke } from "@tauri-apps/api/core";
import i18next from "i18next";
import { initReactI18next } from "react-i18next";

import en from "./locales/en";
import id from "./locales/id";
import { checkPack, type Resources } from "./pack";

export const LANGUAGE_STORAGE_KEY = "zagzig:language";

/** One language the app can show — built in, or a custom language pack. */
export interface LanguageInfo {
  code: string;
  name: string;
  builtIn: boolean;
  dir: "ltr" | "rtl";
  /** Share of the app's strings this language translates, 0–1. */
  coverage: number;
  /** Strings dropped because their placeholders didn't match English. */
  mismatched: number;
  /** Strings in the pack that the app doesn't have. */
  unknown: number;
}

export interface PackLoadError {
  file: string;
  error: string;
}

interface RawPack {
  code: string;
  name: string;
  dir: string;
  resources: Resources;
}

function builtIn(code: string, name: string, resources: Resources): LanguageInfo {
  const check = checkPack(resources, en as Resources);
  return {
    code,
    name,
    builtIn: true,
    dir: "ltr",
    coverage: check.translated / check.total,
    mismatched: check.mismatched.length,
    unknown: check.unknown,
  };
}

const BUILT_IN_LANGUAGES: LanguageInfo[] = [
  builtIn("en", "English", en as Resources),
  builtIn("id", "Bahasa Indonesia", id as Resources),
];

// The built-in resources, kept so they can be exported as templates.
export const BUILT_IN_RESOURCES: Record<string, Resources> = {
  en: en as Resources,
  id: id as Resources,
};

let languages: LanguageInfo[] = BUILT_IN_LANGUAGES;
let loadErrors: PackLoadError[] = [];
const listeners = new Set<() => void>();

function publish(next: LanguageInfo[], errors: PackLoadError[]) {
  languages = next;
  loadErrors = errors;
  listeners.forEach((l) => l());
}

function subscribe(listener: () => void) {
  listeners.add(listener);
  return () => listeners.delete(listener);
}

/** The current list outside React (e.g. right after a reload). */
export function getLanguagesSnapshot(): LanguageInfo[] {
  return languages;
}

export function useLanguages(): LanguageInfo[] {
  return useSyncExternalStore(subscribe, () => languages);
}

export function usePackErrors(): PackLoadError[] {
  return useSyncExternalStore(subscribe, () => loadErrors);
}

function storedLanguage(): string {
  try {
    return localStorage.getItem(LANGUAGE_STORAGE_KEY) || "en";
  } catch {
    return "en"; // localStorage unavailable
  }
}

function applyDocumentLanguage(code: string) {
  const info = languages.find((l) => l.code === code);
  document.documentElement.lang = code;
  document.documentElement.dir = info?.dir ?? "ltr";
}

i18next.use(initReactI18next).init({
  resources: {
    en: { translation: en },
    id: { translation: id },
  },
  // A custom language isn't loaded yet at this point; until its pack
  // arrives (loadLanguagePacks) everything falls back to English.
  lng: storedLanguage(),
  fallbackLng: "en",
  interpolation: { escapeValue: false },
});

i18next.on("languageChanged", applyDocumentLanguage);
applyDocumentLanguage(i18next.language);

export function setLanguage(code: string) {
  void i18next.changeLanguage(code);
  try {
    localStorage.setItem(LANGUAGE_STORAGE_KEY, code);
  } catch {
    // best-effort persistence
  }
}

/**
 * Reads the language packs from the app's languages folder and makes them
 * available. Safe to call again after the folder changes; packs that are
 * gone are unloaded. If the language in use no longer exists, it falls back
 * to English.
 */
export async function loadLanguagePacks(): Promise<void> {
  let result: { packs: RawPack[]; errors: PackLoadError[] };
  try {
    result = await invoke("list_language_packs");
  } catch {
    return; // not running inside Tauri (or the folder is unreadable)
  }

  const known = new Set(BUILT_IN_LANGUAGES.map((l) => l.code));
  const custom: LanguageInfo[] = [];
  for (const pack of result.packs) {
    const check = checkPack(pack.resources, en as Resources);
    i18next.addResourceBundle(pack.code, "translation", check.resources, true, true);
    known.add(pack.code);
    custom.push({
      code: pack.code,
      name: pack.name,
      builtIn: false,
      dir: pack.dir === "rtl" ? "rtl" : "ltr",
      coverage: check.translated / check.total,
      mismatched: check.mismatched.length,
      unknown: check.unknown,
    });
  }

  // Unload custom languages whose pack was removed.
  for (const code of languages.filter((l) => !l.builtIn).map((l) => l.code)) {
    if (!known.has(code)) i18next.removeResourceBundle(code, "translation");
  }

  publish([...BUILT_IN_LANGUAGES, ...custom.sort((a, b) => a.name.localeCompare(b.name))], result.errors);

  const current = i18next.language;
  if (!known.has(current)) {
    setLanguage("en");
  } else {
    // Re-announce the language so components re-render with the new bundle
    // and the document's lang/dir are refreshed.
    void i18next.changeLanguage(current);
  }
}

export default i18next;
