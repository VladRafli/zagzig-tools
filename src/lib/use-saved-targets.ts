import { useCallback, useState } from "react";

// Named hosts the user checks often. One list shared by Connection Test,
// Port Scanner and TLS Inspector: each page saves the part it knows about
// (the host, a port list, a TLS port) and an entry with the same name is
// updated rather than duplicated, so a "staging" saved from one page picks
// up its ports from another.
const STORAGE_KEY = "zagzig:saved-targets";
const MAX_TARGETS = 50;

export interface SavedTarget {
  id: string;
  name: string;
  host: string;
  /** Port list for the Port Scanner, e.g. "22,80,443". */
  ports?: string;
  /** Port for the TLS Inspector. */
  tlsPort?: string;
}

function read(): SavedTarget[] {
  try {
    const parsed = JSON.parse(localStorage.getItem(STORAGE_KEY) ?? "[]");
    return Array.isArray(parsed)
      ? parsed.filter((t) => t && typeof t.id === "string" && typeof t.name === "string" && typeof t.host === "string")
      : [];
  } catch {
    return [];
  }
}

function write(targets: SavedTarget[]) {
  try {
    localStorage.setItem(STORAGE_KEY, JSON.stringify(targets));
  } catch {
    // best-effort, like the other local lists
  }
}

/** Merges `incoming` into `list`: same name (ignoring case) updates that entry. */
export function upsertTarget(list: SavedTarget[], incoming: Omit<SavedTarget, "id">): SavedTarget[] {
  const name = incoming.name.trim();
  const host = incoming.host.trim();
  const existing = list.find((t) => t.name.toLowerCase() === name.toLowerCase());
  const defined = <T,>(value: T | undefined) => (value === undefined || value === "" ? undefined : value);
  if (existing) {
    const merged: SavedTarget = {
      ...existing,
      name,
      host,
      ports: defined(incoming.ports) ?? existing.ports,
      tlsPort: defined(incoming.tlsPort) ?? existing.tlsPort,
    };
    return list.map((t) => (t.id === existing.id ? merged : t));
  }
  const created: SavedTarget = {
    id: `${Date.now().toString(36)}-${Math.random().toString(36).slice(2, 7)}`,
    name,
    host,
    ports: defined(incoming.ports),
    tlsPort: defined(incoming.tlsPort),
  };
  return [created, ...list].slice(0, MAX_TARGETS);
}

export function useSavedTargets() {
  const [targets, setTargets] = useState<SavedTarget[]>(read);

  const save = useCallback((incoming: Omit<SavedTarget, "id">) => {
    setTargets((prev) => {
      const next = upsertTarget(prev, incoming);
      write(next);
      return next;
    });
  }, []);

  const remove = useCallback((id: string) => {
    setTargets((prev) => {
      const next = prev.filter((t) => t.id !== id);
      write(next);
      return next;
    });
  }, []);

  return { targets, save, remove };
}
