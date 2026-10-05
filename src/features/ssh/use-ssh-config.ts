import { useCachedInvoke } from "@/lib/use-cached-invoke";

const CACHE_KEY = "zagzig:ssh-config";

export interface SshHost {
  lineNumber: number;
  patterns: string[];
  hostName: string | null;
  user: string | null;
  port: string | null;
  identityFile: string | null;
  proxyJump: string | null;
  otherOptions: number;
}

export interface SshConfig {
  raw: string;
  hosts: SshHost[];
}

export function useSshConfig() {
  const { data, status, error, updatedAt, refresh } = useCachedInvoke<SshConfig>(
    CACHE_KEY,
    "get_ssh_hosts",
  );

  return {
    raw: data?.raw ?? "",
    hosts: data?.hosts ?? [],
    status,
    error,
    updatedAt,
    refresh,
  };
}
