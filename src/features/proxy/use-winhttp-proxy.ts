import { useCachedInvoke } from "@/lib/use-cached-invoke";

const CACHE_KEY = "zagzig:winhttp-proxy";

export interface WinHttpProxy {
  enabled: boolean;
  proxyServer: string | null;
  bypassList: string | null;
}

export function useWinHttpProxy() {
  const { data, status, error, updatedAt, refresh } = useCachedInvoke<WinHttpProxy>(
    CACHE_KEY,
    "get_winhttp_proxy",
  );

  return {
    proxy: data,
    status,
    error,
    updatedAt,
    refresh,
  };
}
