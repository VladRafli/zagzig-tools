import { useSyncExternalStore } from "react";

import { navGroups, type NavId } from "@/lib/nav";

// Features the user starred for quick access. Shared by the Dashboard and
// the sidebar through a tiny external store, so both update together.
const STORAGE_KEY = "zagzig:starred";
const VALID = new Set<string>(
  navGroups.flatMap((g) => g.items.map((i) => i.id)).filter((id) => id !== "dashboard"),
);

function read(): NavId[] {
  try {
    const parsed = JSON.parse(localStorage.getItem(STORAGE_KEY) ?? "[]");
    return Array.isArray(parsed)
      ? (parsed.filter((id) => typeof id === "string" && VALID.has(id)) as NavId[])
      : [];
  } catch {
    return [];
  }
}

let current = read();
const listeners = new Set<() => void>();

function subscribe(listener: () => void) {
  listeners.add(listener);
  return () => listeners.delete(listener);
}

export function toggleStarred(id: NavId) {
  current = current.includes(id) ? current.filter((x) => x !== id) : [...current, id];
  try {
    localStorage.setItem(STORAGE_KEY, JSON.stringify(current));
  } catch {
    // Storage unavailable: the star still works until the app closes.
  }
  listeners.forEach((l) => l());
}

export function useStarred(): NavId[] {
  return useSyncExternalStore(subscribe, () => current);
}
