import { useCachedInvoke } from "@/lib/use-cached-invoke";

const CACHE_KEY = "zagzig:ports";

// Connections change constantly, so a stale snapshot is worse than a slow
// one — much shorter than the default cache lifetime.
const TTL_MS = 10 * 1000;

export interface PortEntry {
  protocol: "TCP" | "UDP";
  localAddress: string;
  localPort: number;
  remoteAddress: string;
  remotePort: number;
  state: string;
  pid: number;
}

export interface PortProcess {
  pid: number;
  name: string;
  path: string | null;
  commandLine: string | null;
  parentPid: number | null;
  parentName: string | null;
  startTime: string | null;
  company: string | null;
  description: string | null;
  version: string | null;
  workingSetMb: number | null;
  services: string[];
}

interface PortsSnapshot {
  entries: PortEntry[];
  processes: PortProcess[];
}

export function usePorts() {
  const { data, status, error, updatedAt, refresh } =
    useCachedInvoke<PortsSnapshot>(CACHE_KEY, "get_port_usage", TTL_MS);

  return {
    entries: data?.entries ?? [],
    processes: data?.processes ?? [],
    status,
    error,
    updatedAt,
    refresh,
  };
}
