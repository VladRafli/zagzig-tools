import { useCachedInvoke } from "@/lib/use-cached-invoke";

const CACHE_KEY = "zagzig:firewall";

export interface FirewallRule {
  name: string;
  displayName: string;
  enabled: boolean;
  direction: "Inbound" | "Outbound";
  action: "Allow" | "Block";
  profile: string;
  protocol: string;
  localPort: string;
  remotePort: string;
  program: string | null;
  group: string | null;
  appCreated?: boolean;
}

export interface FirewallProfile {
  name: string;
  enabled: boolean;
}

interface FirewallSnapshot {
  profiles: FirewallProfile[];
  rules: FirewallRule[];
}

export function useFirewall() {
  const { data, status, error, updatedAt, refresh } =
    useCachedInvoke<FirewallSnapshot>(CACHE_KEY, "get_firewall", 60 * 1000);

  return {
    profiles: data?.profiles ?? [],
    rules: data?.rules ?? [],
    status,
    error,
    updatedAt,
    refresh,
  };
}
