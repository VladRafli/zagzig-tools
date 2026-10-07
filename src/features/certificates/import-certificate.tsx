import { useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";
import { FileUp, KeyRound, Loader2, ShieldAlert } from "lucide-react";
import { useTranslation } from "react-i18next";
import { toast } from "sonner";

import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import {
  Select,
  SelectContent,
  SelectGroup,
  SelectItem,
  SelectLabel,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { CERT_STORE_OPTIONS, storeOptionKey } from "@/features/certificates/use-certificates";
import { useIsAdministrator } from "@/lib/use-is-administrator";

interface FileCert {
  subject: string;
  issuer: string;
  thumbprint: string;
  notBefore: string;
  notAfter: string;
  selfSigned: boolean;
  isCa: boolean;
  hasPrivateKey: boolean;
}

interface FileInfo {
  certs: FileCert[];
  needsPassword: boolean;
  error: string | null;
}

type StoreOption = (typeof CERT_STORE_OPTIONS)[number];

function errorMessage(err: unknown) {
  return err instanceof Error ? err.message : String(err);
}

function formatDate(iso: string) {
  const date = new Date(iso);
  return Number.isNaN(date.getTime()) ? iso : date.toLocaleDateString();
}

export function ImportCertificate({
  defaultStore,
  onImported,
}: {
  defaultStore: StoreOption;
  onImported: () => void;
}) {
  const { t } = useTranslation();
  const { isAdministrator } = useIsAdministrator();

  const [path, setPath] = useState<string | null>(null);
  const [password, setPassword] = useState("");
  const [info, setInfo] = useState<FileInfo | null>(null);
  const [destination, setDestination] = useState<StoreOption>(defaultStore);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const fileName = path ? path.split(/[\\/]/).pop() : "";
  const isRoot = destination.store === "Root";

  function close() {
    if (busy) return;
    setPath(null);
    setPassword("");
    setInfo(null);
    setError(null);
  }

  async function inspect(file: string, pw: string) {
    setBusy(true);
    setError(null);
    try {
      const result = await invoke<FileInfo>("inspect_certificate_file", {
        path: file,
        password: pw || null,
      });
      setInfo(result);
      if (result.error) setError(result.error);
      else if (result.needsPassword && pw) setError(t("certificates.import.wrongPassword"));
    } catch (err) {
      setInfo(null);
      setError(errorMessage(err));
    } finally {
      setBusy(false);
    }
  }

  async function choose() {
    const picked = await open({
      multiple: false,
      directory: false,
      filters: [
        {
          name: t("certificates.import.fileFilter"),
          extensions: ["cer", "crt", "der", "pem", "p7b", "pfx", "p12"],
        },
      ],
    });
    if (typeof picked !== "string") return;
    setDestination(defaultStore);
    setPassword("");
    setInfo(null);
    setPath(picked);
    await inspect(picked, "");
  }

  async function confirm() {
    if (!path) return;
    setBusy(true);
    setError(null);
    try {
      const result = await invoke<{ count: number }>("import_certificate", {
        scope: destination.scope,
        store: destination.store,
        path,
        password: password || null,
      });
      toast.success(t("certificates.import.success", { count: result.count }));
      setBusy(false);
      onImported();
      close();
    } catch (err) {
      setError(errorMessage(err));
      setBusy(false);
    }
  }

  const needsPassword = info?.needsPassword ?? false;
  const certs = info?.certs ?? [];
  const canImport = certs.length > 0 && !busy && (isAdministrator || destination.scope !== "LocalMachine");
  const currentUserOptions = CERT_STORE_OPTIONS.filter((o) => o.scope === "CurrentUser");
  const localMachineOptions = CERT_STORE_OPTIONS.filter((o) => o.scope === "LocalMachine");

  return (
    <>
      <Button variant="outline" onClick={choose} disabled={busy}>
        <FileUp />
        {t("certificates.import.button")}
      </Button>

      <Dialog open={path !== null} onOpenChange={(next) => !next && close()}>
        <DialogContent className="sm:max-w-xl">
          <DialogHeader>
            <DialogTitle>{t("certificates.import.title")}</DialogTitle>
            <DialogDescription>{fileName}</DialogDescription>
          </DialogHeader>

          {busy && !info && (
            <p className="flex items-center gap-2 text-sm text-muted-foreground">
              <Loader2 className="size-4 animate-spin" />
              {t("certificates.import.reading")}
            </p>
          )}

          {needsPassword && (
            <div className="flex flex-col gap-2">
              <Label htmlFor="certPassword" className="flex items-center gap-1.5">
                <KeyRound className="size-4" />
                {t("certificates.import.passwordLabel")}
              </Label>
              <div className="flex gap-2">
                <Input
                  id="certPassword"
                  type="password"
                  value={password}
                  onChange={(e) => setPassword(e.currentTarget.value)}
                  onKeyDown={(e) => e.key === "Enter" && path && inspect(path, password)}
                  autoFocus
                />
                <Button onClick={() => path && inspect(path, password)} disabled={busy || !password}>
                  {busy && <Loader2 className="animate-spin" />}
                  {t("certificates.import.unlock")}
                </Button>
              </div>
              <p className="text-xs text-muted-foreground">{t("certificates.import.passwordHint")}</p>
            </div>
          )}

          {certs.length > 0 && (
            <>
              <div className="max-h-56 overflow-y-auto rounded-lg border">
                {certs.map((c) => (
                  <div key={c.thumbprint} className="flex flex-col gap-1 border-b px-3 py-2 text-sm last:border-b-0">
                    <span className="font-medium break-all">{c.subject}</span>
                    <span className="text-xs text-muted-foreground">
                      {t("certificates.import.issuedBy", { issuer: c.selfSigned ? t("certificates.import.itself") : c.issuer })} ·{" "}
                      {t("certificates.import.expires", { date: formatDate(c.notAfter) })}
                    </span>
                    <div className="flex flex-wrap gap-1">
                      {c.isCa && <Badge variant="secondary">{t("certificates.import.ca")}</Badge>}
                      {c.selfSigned && <Badge variant="outline">{t("certificates.import.selfSigned")}</Badge>}
                      {c.hasPrivateKey && <Badge variant="outline">{t("certificates.import.hasKey")}</Badge>}
                    </div>
                    <code className="text-xs text-muted-foreground">{c.thumbprint}</code>
                  </div>
                ))}
              </div>

              <div className="flex flex-col gap-1.5">
                <Label>{t("certificates.import.destination")}</Label>
                <Select
                  value={storeOptionKey(destination)}
                  onValueChange={(value) => {
                    const next = CERT_STORE_OPTIONS.find((o) => storeOptionKey(o) === value);
                    if (next) setDestination(next);
                  }}
                >
                  <SelectTrigger className="w-full">
                    <SelectValue>{() => t(destination.labelKey)}</SelectValue>
                  </SelectTrigger>
                  <SelectContent>
                    <SelectGroup>
                      <SelectLabel>{t("common.currentUser")}</SelectLabel>
                      {currentUserOptions.map((o) => (
                        <SelectItem key={storeOptionKey(o)} value={storeOptionKey(o)}>
                          {t(o.labelKey)}
                        </SelectItem>
                      ))}
                    </SelectGroup>
                    <SelectGroup>
                      <SelectLabel>{t("common.localMachine")}</SelectLabel>
                      {localMachineOptions.map((o) => (
                        <SelectItem key={storeOptionKey(o)} value={storeOptionKey(o)} disabled={!isAdministrator}>
                          {t(o.labelKey)}
                        </SelectItem>
                      ))}
                    </SelectGroup>
                  </SelectContent>
                </Select>
              </div>

              {isRoot && (
                <div className="flex gap-2 rounded-lg border border-destructive/40 bg-destructive/5 p-3 text-sm">
                  <ShieldAlert className="mt-0.5 size-4 shrink-0 text-destructive" />
                  <div>
                    <p className="font-medium text-destructive">{t("certificates.import.rootWarningTitle")}</p>
                    <p className="mt-1 text-muted-foreground">
                      {t("certificates.import.rootWarning", {
                        who: destination.scope === "LocalMachine" ? t("certificates.import.everyone") : t("certificates.import.you"),
                      })}
                    </p>
                    {destination.scope === "CurrentUser" && (
                      <p className="mt-1 text-xs text-muted-foreground">{t("certificates.import.windowsPrompt")}</p>
                    )}
                  </div>
                </div>
              )}
              {destination.scope === "LocalMachine" && (
                <p className="text-xs text-muted-foreground">{t("certificates.import.adminNote")}</p>
              )}
            </>
          )}

          {error && <p className="text-sm text-destructive">{error}</p>}

          <DialogFooter>
            <Button variant="outline" onClick={close} disabled={busy}>
              {t("certificates.import.cancel")}
            </Button>
            <Button onClick={confirm} disabled={!canImport} variant={isRoot ? "destructive" : "default"}>
              {busy && info && <Loader2 className="animate-spin" />}
              {t("certificates.import.confirm", { count: certs.length })}
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </>
  );
}
