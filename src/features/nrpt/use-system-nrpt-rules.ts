import { useCachedInvoke } from "@/lib/use-cached-invoke";

// Bumped to v2 to invalidate any rule data cached before NameServers was
// fixed to serialize as plain strings instead of empty (see get_nrpt_rules).
const CACHE_KEY = "zagzig:nrpt-rules:v2";

export interface SystemNrptRule {
  name: string;
  displayName: string | null;
  comment: string | null;
  namespace: string[];
  nameServers: string[];
  nameEncoding: string | null;
  version: number | null;
  dnsSecEnabled: boolean;
  dnsSecValidationRequired: boolean | null;
  dnsSecQueryIpsecEncryption: string | null;
  dnsSecQueryIpsecRequired: boolean | null;
  directAccessEnabled: boolean;
  directAccessDnsServers: string[];
  directAccessProxyName: string | null;
  directAccessProxyType: string | null;
  directAccessQueryIpsecEncryption: string | null;
  directAccessQueryIpsecRequired: boolean | null;
  ipsecCaRestriction: string | null;
}

export function useSystemNrptRules() {
  const { data, status, error, updatedAt, refresh } = useCachedInvoke<
    SystemNrptRule[]
  >(CACHE_KEY, "get_nrpt_rules");

  return { rules: data ?? [], status, error, updatedAt, refresh };
}
