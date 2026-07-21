import { useCachedInvoke } from "@/lib/use-cached-invoke";

const CACHE_KEY = "zagzig:network-routes";

export interface NetRoute {
  destinationPrefix: string;
  nextHop: string;
  interfaceAlias: string;
  interfaceIndex: number;
  routeMetric: number;
  interfaceMetric: number;
  protocol: string;
  store: string;
}

export function useNetworkRoutes() {
  const { data, status, error, updatedAt, refresh } = useCachedInvoke<
    NetRoute[]
  >(CACHE_KEY, "get_routes");

  return { routes: data ?? [], status, error, updatedAt, refresh };
}
