import { useCachedInvoke } from "@/lib/use-cached-invoke";

const CACHE_KEY = "zagzig:wsl-status";

export interface WslDistro {
  name: string;
  running: boolean;
  version: string;
  isDefault: boolean;
}

export interface WslSettings {
  memory: string | null;
  processors: string | null;
  swap: string | null;
  localhostForwarding: string | null;
  networkingMode: string | null;
  autoMemoryReclaim: string | null;
  nestedVirtualization: string | null;
}

export interface DockerDesktop {
  installed: boolean;
  running: boolean;
}

export interface WslStatus {
  installed: boolean;
  unresponsive: boolean;
  dockerDesktop: DockerDesktop;
  distros: WslDistro[];
  settings: WslSettings;
  rawConfig: string;
}

export function useWsl() {
  const { data, status, error, updatedAt, refresh } = useCachedInvoke<WslStatus>(
    CACHE_KEY,
    "get_wsl_status",
  );

  return {
    installed: data?.installed ?? true,
    unresponsive: data?.unresponsive ?? false,
    dockerDesktop: data?.dockerDesktop ?? { installed: false, running: false },
    distros: data?.distros ?? [],
    settings: data?.settings ?? null,
    rawConfig: data?.rawConfig ?? "",
    loaded: data !== null && data !== undefined,
    status,
    error,
    updatedAt,
    refresh,
  };
}
