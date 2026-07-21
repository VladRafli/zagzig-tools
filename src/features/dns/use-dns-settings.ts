import { useCachedInvoke } from "@/lib/use-cached-invoke";

const CACHE_KEY = "zagzig:dns-settings";

export interface DnsInterface {
  interfaceAlias: string;
  interfaceIndex: number;
  serverAddresses: string[];
  dhcp: boolean;
  status: string;
}

export function useDnsSettings() {
  const { data, status, error, updatedAt, refresh } = useCachedInvoke<
    DnsInterface[]
  >(CACHE_KEY, "get_dns_settings");

  return { interfaces: data ?? [], status, error, updatedAt, refresh };
}
