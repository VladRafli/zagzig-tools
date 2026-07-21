import { useCallback, useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";

export interface CertificateDetail {
  thumbprint: string;
  subject: string;
  issuer: string;
  serialNumber: string;
  friendlyName: string;
  notBefore: string;
  notAfter: string;
  hasPrivateKey: boolean;
  isExpired: boolean;
  enhancedKeyUsages: string[];
}

export type CertScope = "CurrentUser" | "LocalMachine";
export type CertStoreName = "My" | "Root" | "CA" | "TrustedPublisher";

export interface CertStoreOption {
  scope: CertScope;
  store: CertStoreName;
  labelKey: string;
}

// Mirrors the allowlist in `certificates::cert_store_path` on the Rust side —
// these are the only (scope, store) pairs the backend accepts.
export const CERT_STORE_OPTIONS: CertStoreOption[] = [
  { scope: "CurrentUser", store: "My", labelKey: "certificates.stores.currentUserMy" },
  { scope: "CurrentUser", store: "Root", labelKey: "certificates.stores.currentUserRoot" },
  { scope: "CurrentUser", store: "CA", labelKey: "certificates.stores.currentUserCa" },
  {
    scope: "CurrentUser",
    store: "TrustedPublisher",
    labelKey: "certificates.stores.currentUserTrustedPublisher",
  },
  { scope: "LocalMachine", store: "My", labelKey: "certificates.stores.localMachineMy" },
  { scope: "LocalMachine", store: "Root", labelKey: "certificates.stores.localMachineRoot" },
  { scope: "LocalMachine", store: "CA", labelKey: "certificates.stores.localMachineCa" },
  {
    scope: "LocalMachine",
    store: "TrustedPublisher",
    labelKey: "certificates.stores.localMachineTrustedPublisher",
  },
];

export function storeOptionKey(option: { scope: string; store: string }): string {
  return `${option.scope}/${option.store}`;
}

type QueryStatus = "loading" | "ready" | "error";

// Deliberately not routed through the shared localStorage-backed cache: the
// query is parameterized by which store is selected, switching stores should
// always show fresh data, and a certificate store listing is cheap enough
// that there's little to gain from caching it across app restarts.
export function useCertificates(scope: CertScope, store: CertStoreName) {
  const [certificates, setCertificates] = useState<CertificateDetail[]>([]);
  const [status, setStatus] = useState<QueryStatus>("loading");
  const [error, setError] = useState<string | null>(null);

  const refresh = useCallback(() => {
    setStatus("loading");
    setError(null);
    invoke<CertificateDetail[]>("get_certificates", { scope, store })
      .then((result) => {
        setCertificates(result);
        setStatus("ready");
      })
      .catch((err) => {
        setError(err instanceof Error ? err.message : String(err));
        setStatus("error");
      });
  }, [scope, store]);

  useEffect(() => {
    refresh();
  }, [refresh]);

  return { certificates, status, error, refresh };
}
