import { useCachedInvoke } from "@/lib/use-cached-invoke";

const CACHE_KEY = "zagzig:port-proxy";

export type PortProxyKind = "v4tov4" | "v4tov6" | "v6tov4" | "v6tov6";

export const PORT_PROXY_KINDS: PortProxyKind[] = [
  "v4tov4",
  "v4tov6",
  "v6tov4",
  "v6tov6",
];

export interface PortProxyRule {
  kind: PortProxyKind;
  listenAddress: string;
  listenPort: number;
  connectAddress: string;
  connectPort: number;
}

export function usePortProxy() {
  const { data, status, error, updatedAt, refresh } = useCachedInvoke<
    PortProxyRule[]
  >(CACHE_KEY, "get_portproxy_rules");

  return { rules: data ?? [], status, error, updatedAt, refresh };
}
