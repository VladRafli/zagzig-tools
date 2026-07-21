import { useState } from "react";

const HISTORY_KEY = "zagzig:connection-test-history";
const MAX_ENTRIES = 15;

export interface ConnectionHistoryEntry {
  id: string;
  target: string;
  timestamp: number;
  reachable: boolean | null;
  avgMs: number | null;
  lossPercent: number | null;
  hopCount: number | null;
  pingError: string | null;
  traceError: string | null;
}

function readHistory(): ConnectionHistoryEntry[] {
  try {
    const raw = localStorage.getItem(HISTORY_KEY);
    return raw ? (JSON.parse(raw) as ConnectionHistoryEntry[]) : [];
  } catch {
    return [];
  }
}

function writeHistory(entries: ConnectionHistoryEntry[]) {
  try {
    localStorage.setItem(HISTORY_KEY, JSON.stringify(entries));
  } catch {
    // ignore — history is best-effort
  }
}

export function useConnectionHistory() {
  const [entries, setEntries] = useState<ConnectionHistoryEntry[]>(readHistory);

  function addEntry(entry: Omit<ConnectionHistoryEntry, "id" | "timestamp">) {
    setEntries((prev) => {
      const next = [
        { ...entry, id: crypto.randomUUID(), timestamp: Date.now() },
        ...prev,
      ].slice(0, MAX_ENTRIES);
      writeHistory(next);
      return next;
    });
  }

  function clear() {
    setEntries([]);
    writeHistory([]);
  }

  return { entries, addEntry, clear };
}
