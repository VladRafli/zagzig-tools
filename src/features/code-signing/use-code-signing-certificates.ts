import { useCachedInvoke } from "@/lib/use-cached-invoke";

const CACHE_KEY = "zagzig:code-signing-certificates";

export interface CodeSigningCertificate {
  thumbprint: string;
  subject: string;
  issuer: string;
  notAfter: string;
  hasPrivateKey: boolean;
}

export function useCodeSigningCertificates() {
  const { data, status, error, refresh } = useCachedInvoke<
    CodeSigningCertificate[]
  >(CACHE_KEY, "list_code_signing_certificates");

  return { certificates: data ?? [], status, error, refresh };
}
