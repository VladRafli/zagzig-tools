export interface CacheEntry<T> {
  data: T;
  fetchedAt: number;
}

// localStorage access is wrapped in try/catch since it can throw (private
// browsing quotas, disabled storage) — the cache is always best-effort, and
// a failure here should never break the feature it's backing.

export function readCacheEntry<T>(key: string): CacheEntry<T> | null {
  try {
    const raw = localStorage.getItem(key);
    return raw ? (JSON.parse(raw) as CacheEntry<T>) : null;
  } catch {
    return null;
  }
}

export function writeCacheEntry<T>(key: string, data: T): number {
  const fetchedAt = Date.now();
  try {
    localStorage.setItem(key, JSON.stringify({ data, fetchedAt }));
  } catch {
    // ignore — cache is best-effort
  }
  return fetchedAt;
}
