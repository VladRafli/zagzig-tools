import { useCachedInvoke } from "@/lib/use-cached-invoke";

const CACHE_KEY = "zagzig:hosts-file";

export interface HostsEntry {
  lineNumber: number;
  enabled: boolean;
  ip: string;
  hostnames: string[];
  comment: string | null;
}

export interface HostsFile {
  raw: string;
  entries: HostsEntry[];
}

export function useHostsFile() {
  const { data, status, error, updatedAt, refresh } = useCachedInvoke<HostsFile>(
    CACHE_KEY,
    "get_hosts_entries",
  );

  return {
    raw: data?.raw ?? "",
    entries: data?.entries ?? [],
    status,
    error,
    updatedAt,
    refresh,
  };
}
