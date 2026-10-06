// Pure helpers for language packs: checking that a translation is safe to
// load (its placeholders match the English source), and how much of the app
// it covers. No i18next or Tauri here, so it can be tested on its own.

export interface Resources {
  [key: string]: string | Resources;
}

export interface PackCheck {
  /** The strings that are safe to load, in the same nested shape. */
  resources: Resources;
  /** Number of strings in the base (English) language. */
  total: number;
  /** How many of those the pack translates correctly. */
  translated: number;
  /** Keys dropped because their placeholders differ from the English text. */
  mismatched: string[];
  /** Keys in the pack that don't exist in the app (typos, old versions). */
  unknown: number;
}

const SEPARATOR = ".";
// i18next picks a plural form by suffix; languages need different sets of
// them (Russian _few/_many, Arabic _zero/_two, ...), so forms the English
// source doesn't have are still valid.
const PLURAL_SUFFIX = /_(zero|one|two|few|many|other)$/;

export function flattenStrings(resources: Resources, prefix = ""): Map<string, string> {
  const out = new Map<string, string>();
  for (const [key, value] of Object.entries(resources)) {
    const path = prefix ? `${prefix}${SEPARATOR}${key}` : key;
    if (typeof value === "string") out.set(path, value);
    else if (value && typeof value === "object") {
      for (const [k, v] of flattenStrings(value, path)) out.set(k, v);
    }
  }
  return out;
}

function unflatten(flat: Map<string, string>): Resources {
  const root: Resources = {};
  for (const [path, value] of flat) {
    const parts = path.split(SEPARATOR);
    let node = root;
    for (const part of parts.slice(0, -1)) {
      const next = node[part];
      if (typeof next === "object") node = next;
      else {
        const created: Resources = {};
        node[part] = created;
        node = created;
      }
    }
    node[parts[parts.length - 1]] = value;
  }
  return root;
}

/** The interpolation values and numbered tags a string relies on. */
export function placeholdersOf(text: string): string[] {
  const found = new Set<string>();
  for (const m of text.matchAll(/\{\{\s*([^},\s]+)[^}]*\}\}/g)) found.add(`{{${m[1]}}}`);
  for (const m of text.matchAll(/<\/?\s*(\d+)\s*\/?>/g)) found.add(`<${m[1]}>`);
  return [...found].sort();
}

function samePlaceholders(a: string, b: string): boolean {
  const pa = placeholdersOf(a);
  const pb = placeholdersOf(b);
  return pa.length === pb.length && pa.every((p, i) => p === pb[i]);
}

export function checkPack(pack: Resources, base: Resources): PackCheck {
  const baseFlat = flattenStrings(base);
  const accepted = new Map<string, string>();
  const mismatched: string[] = [];
  let unknown = 0;
  let translated = 0;

  for (const [key, value] of flattenStrings(pack)) {
    const baseText = baseFlat.get(key);
    if (baseText !== undefined) {
      if (samePlaceholders(value, baseText)) {
        accepted.set(key, value);
        translated += 1;
      } else {
        mismatched.push(key);
      }
      continue;
    }
    // Not in the app as such — fine if it's an extra plural form of a key
    // that is, as long as it keeps that key's placeholders.
    const reference = PLURAL_SUFFIX.test(key)
      ? baseFlat.get(key.replace(PLURAL_SUFFIX, "_other"))
      : undefined;
    if (reference === undefined) {
      unknown += 1;
    } else if (samePlaceholders(value, reference)) {
      accepted.set(key, value);
    } else {
      mismatched.push(key);
    }
  }

  return {
    resources: unflatten(accepted),
    total: baseFlat.size,
    translated,
    mismatched,
    unknown,
  };
}

/** A starting point for translators: `$meta` to fill in, then the strings. */
export function buildTemplate(resources: Resources): string {
  return `${JSON.stringify(
    { $meta: { code: "xx", name: "Your language name", dir: "ltr" }, ...resources },
    null,
    2,
  )}\n`;
}
