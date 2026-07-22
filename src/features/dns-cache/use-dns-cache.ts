import { useCachedInvoke } from "@/lib/use-cached-invoke";

const CACHE_KEY = "zagzig:dns-cache";

export interface DnsCacheEntry {
  name: string;
  recordType: string;
  data: string | null;
  timeToLive: number;
  section: string;
  status: string | null;
}

export function useDnsCache() {
  const { data, status, error, updatedAt, refresh } = useCachedInvoke<
    DnsCacheEntry[]
  >(CACHE_KEY, "get_dns_cache");

  return { entries: data ?? [], status, error, updatedAt, refresh };
}
