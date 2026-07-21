import { useCallback, useEffect, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";

import { readCacheEntry, writeCacheEntry } from "@/lib/local-storage-cache";

const DEFAULT_TTL_MS = 5 * 60 * 1000;

export type QueryStatus = "loading" | "ready" | "error";

// Wraps a Tauri `invoke` call with a localStorage-backed cache, so
// navigating away and back (or restarting the app) doesn't re-run a slow
// PowerShell/LDAP/network call within the TTL. `refresh()` always bypasses
// the cache.
export function useCachedInvoke<T>(
  cacheKey: string,
  command: string,
  ttlMs: number = DEFAULT_TTL_MS,
) {
  const cached = readCacheEntry<T>(cacheKey);
  const isFresh = cached !== null && Date.now() - cached.fetchedAt < ttlMs;

  const [data, setData] = useState<T | null>(cached?.data ?? null);
  const [status, setStatus] = useState<QueryStatus>(
    isFresh ? "ready" : "loading",
  );
  const [error, setError] = useState<string | null>(null);
  const [updatedAt, setUpdatedAt] = useState<number | null>(
    cached?.fetchedAt ?? null,
  );
  const mountedRef = useRef(true);

  useEffect(() => {
    mountedRef.current = true;
    return () => {
      mountedRef.current = false;
    };
  }, []);

  const refresh = useCallback(() => {
    setStatus("loading");
    setError(null);

    invoke<T>(command)
      .then((result) => {
        if (!mountedRef.current) return;
        setData(result);
        setStatus("ready");
        setUpdatedAt(writeCacheEntry(cacheKey, result));
      })
      .catch((err) => {
        if (!mountedRef.current) return;
        setError(err instanceof Error ? err.message : String(err));
        setStatus("error");
      });
  }, [cacheKey, command]);

  useEffect(() => {
    if (!isFresh) refresh();
    // Only run on mount / when the cache key or command identity changes —
    // `refresh` itself is stable across those.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [cacheKey, command]);

  return { data, status, error, updatedAt, refresh };
}
