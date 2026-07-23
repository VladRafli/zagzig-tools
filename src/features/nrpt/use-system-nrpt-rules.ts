import { useCachedInvoke } from "@/lib/use-cached-invoke";

// Bumped to v3 to invalidate any rule data cached before the *actual* fix
// for empty NameServers landed (a40bb92: Windows PowerShell 5.1's
// ConvertTo-Json wraps a Select-Object calculated array property as
// `{"value": [...], "Count": N}` instead of a plain array — the v2 bump
// predated that discovery and didn't fix anything on its own). See
// get_nrpt_rules for the actual fix.
const CACHE_KEY = "zagzig:nrpt-rules:v3";

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
