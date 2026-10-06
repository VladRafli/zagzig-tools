import { useCachedInvoke } from "@/lib/use-cached-invoke";

const CACHE_KEY = "zagzig:network-adapters";

// Traffic counters move constantly, so keep this snapshot short-lived.
const TTL_MS = 15 * 1000;

export interface NetworkAdapter {
  name: string;
  description: string;
  interfaceIndex: number;
  status: string;
  macAddress: string | null;
  linkSpeed: string | null;
  mediaType: string | null;
  isVirtual: boolean;
  ipv4: string[];
  ipv6: string[];
  gateways: string[];
  dnsServers: string[];
  dhcp: boolean;
  mtu: number | null;
  bytesReceived: number;
  bytesSent: number;
}

export function useAdapters() {
  const { data, status, error, updatedAt, refresh } = useCachedInvoke<
    NetworkAdapter[]
  >(CACHE_KEY, "get_network_adapters", TTL_MS);

  return { adapters: data ?? [], status, error, updatedAt, refresh };
}
