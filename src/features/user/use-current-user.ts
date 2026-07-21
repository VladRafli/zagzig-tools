import { useCachedInvoke } from "@/lib/use-cached-invoke";

const CACHE_KEY = "zagzig:current-user";

export interface LdapUser {
  displayName: string | null;
  givenName: string | null;
  surname: string | null;
  emailAddress: string | null;
  voiceTelephoneNumber: string | null;
  description: string | null;
  distinguishedName: string | null;
  userPrincipalName: string | null;
  enabled: boolean | null;
  title: string | null;
  department: string | null;
  company: string | null;
  office: string | null;
  manager: string | null;
}

export interface CurrentUser {
  userName: string;
  domain: string;
  computerName: string;
  isDomainJoined: boolean;
  sid: string | null;
  profilePath: string;
  ldap: LdapUser | null;
  ldapError: string | null;
}

export function useCurrentUser() {
  const { data, status, error, updatedAt, refresh } =
    useCachedInvoke<CurrentUser>(CACHE_KEY, "get_current_user");

  return { user: data, status, error, updatedAt, refresh };
}
