import { useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { save } from "@tauri-apps/plugin-dialog";
import { Download, Eye, Loader2, Lock, RefreshCw, Trash2 } from "lucide-react";
import { useTranslation } from "react-i18next";
import type { TFunction } from "i18next";
import { toast } from "sonner";

import { Button } from "@/components/ui/button";
import { Card, CardContent } from "@/components/ui/card";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
  DialogTrigger,
} from "@/components/ui/dialog";
import { Label } from "@/components/ui/label";
import { Badge } from "@/components/ui/badge";
import {
  Select,
  SelectContent,
  SelectGroup,
  SelectItem,
  SelectLabel,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { DetailList, DetailRow } from "@/components/detail-list";
import { AdminRequiredTooltip } from "@/components/admin-required-tooltip";
import { useIsAdministrator } from "@/lib/use-is-administrator";
import {
  CERT_STORE_OPTIONS,
  storeOptionKey,
  useCertificates,
  type CertificateDetail,
  type CertScope,
  type CertStoreName,
} from "@/features/certificates/use-certificates";

function extractCommonName(name: string): string {
  const match = name
    .split(",")
    .map((part) => part.trim())
    .find((part) => part.startsWith("CN="));
  return match ? match.slice(3) : name;
}

function DetailsDialog({ cert, t }: { cert: CertificateDetail; t: TFunction }) {
  const [open, setOpen] = useState(false);

  return (
    <Dialog open={open} onOpenChange={setOpen}>
      <DialogTrigger
        render={
          <Button
            variant="ghost"
            size="icon-sm"
            aria-label={t("certificates.details.title")}
          />
        }
      >
        <Eye />
      </DialogTrigger>
      <DialogContent>
        <DialogHeader>
          <DialogTitle>{t("certificates.details.title")}</DialogTitle>
          <DialogDescription className="break-all">{cert.subject}</DialogDescription>
        </DialogHeader>
        <DetailList>
          <DetailRow label={t("certificates.columns.issuer")} value={cert.issuer} />
          <DetailRow
            label={t("certificates.details.thumbprint")}
            value={cert.thumbprint}
          />
          <DetailRow
            label={t("certificates.details.serialNumber")}
            value={cert.serialNumber}
          />
          <DetailRow
            label={t("certificates.details.friendlyName")}
            value={cert.friendlyName}
          />
          <DetailRow label={t("certificates.details.notBefore")} value={cert.notBefore} />
          <DetailRow label={t("certificates.details.notAfter")} value={cert.notAfter} />
          <DetailRow label={t("certificates.hasPrivateKey")} value={cert.hasPrivateKey} />
          <DetailRow
            label={t("certificates.columns.usage")}
            value={
              cert.enhancedKeyUsages.length
                ? cert.enhancedKeyUsages
                : t("certificates.anyPurpose")
            }
          />
        </DetailList>
      </DialogContent>
    </Dialog>
  );
}

function ExportButton({
  cert,
  scope,
  store,
  t,
}: {
  cert: CertificateDetail;
  scope: CertScope;
  store: CertStoreName;
  t: TFunction;
}) {
  const [exporting, setExporting] = useState(false);

  async function exportCert() {
    const suggested = `${extractCommonName(cert.subject).replace(/[\\/:*?"<>|]+/g, "_")}.cer`;
    const destination = await save({
      title: t("certificates.exportDialogTitle"),
      defaultPath: suggested,
      filters: [{ name: "Certificate", extensions: ["cer"] }],
    });
    if (!destination) return;

    setExporting(true);
    try {
      await invoke("export_certificate", {
        scope,
        store,
        thumbprint: cert.thumbprint,
        destinationPath: destination,
      });
      toast.success(t("certificates.exportSuccess", { path: destination }));
    } catch (err) {
      const message = err instanceof Error ? err.message : String(err);
      toast.error(t("certificates.exportError", { error: message }));
    } finally {
      setExporting(false);
    }
  }

  return (
    <Button
      variant="ghost"
      size="icon-sm"
      aria-label={t("certificates.export")}
      onClick={exportCert}
      disabled={exporting}
    >
      {exporting ? <Loader2 className="animate-spin" /> : <Download />}
    </Button>
  );
}

function DeleteDialog({
  cert,
  scope,
  store,
  isAdministrator,
  onDeleted,
  t,
}: {
  cert: CertificateDetail;
  scope: CertScope;
  store: CertStoreName;
  isAdministrator: boolean;
  onDeleted: () => void;
  t: TFunction;
}) {
  const [open, setOpen] = useState(false);
  const [removing, setRemoving] = useState(false);
  const [error, setError] = useState<string | null>(null);

  // Only LocalMachine stores need administrator approval to modify —
  // CurrentUser stores belong to the signed-in account and can be changed
  // without elevation, so the lock only applies here when it's actually
  // needed.
  const needsAdmin = scope === "LocalMachine";
  const disabled = needsAdmin && !isAdministrator;

  function handleOpenChange(next: boolean) {
    if (removing) return;
    setOpen(next);
    if (!next) setError(null);
  }

  async function confirmDelete() {
    setRemoving(true);
    setError(null);
    try {
      await invoke("delete_certificate", { scope, store, thumbprint: cert.thumbprint });
      setOpen(false);
      toast.success(t("certificates.remove.success"));
      onDeleted();
    } catch (err) {
      const message = err instanceof Error ? err.message : String(err);
      setError(t("certificates.remove.error", { error: message }));
    } finally {
      setRemoving(false);
    }
  }

  return (
    <Dialog open={open} onOpenChange={handleOpenChange}>
      <AdminRequiredTooltip locked={disabled}>
        <DialogTrigger
          render={
            <Button
              variant="ghost"
              size="icon-sm"
              aria-label={t("certificates.remove.button")}
              disabled={disabled}
            />
          }
        >
          {disabled ? <Lock /> : <Trash2 />}
        </DialogTrigger>
      </AdminRequiredTooltip>
      <DialogContent>
        <DialogHeader>
          <DialogTitle>{t("certificates.remove.title")}</DialogTitle>
          <DialogDescription>
            {t("certificates.remove.description", {
              subject: extractCommonName(cert.subject),
              thumbprint: cert.thumbprint,
            })}
          </DialogDescription>
        </DialogHeader>
        {error && <p className="text-sm text-destructive">{error}</p>}
        <DialogFooter>
          <Button variant="outline" onClick={() => handleOpenChange(false)} disabled={removing}>
            {t("certificates.remove.cancel")}
          </Button>
          <Button variant="destructive" onClick={confirmDelete} disabled={removing}>
            {removing && <Loader2 className="animate-spin" />}
            {removing ? t("certificates.remove.removing") : t("certificates.remove.confirm")}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}

function CertificateTable({
  certificates,
  scope,
  store,
  isAdministrator,
  onChanged,
  t,
}: {
  certificates: CertificateDetail[];
  scope: CertScope;
  store: CertStoreName;
  isAdministrator: boolean;
  onChanged: () => void;
  t: TFunction;
}) {
  return (
    <div className="overflow-x-auto rounded-lg border">
      <div className="min-w-[48rem]">
        <div className="grid grid-cols-[1.4fr_1.2fr_7rem_1fr_7rem] gap-2 border-b bg-muted/50 px-3 py-2 text-xs font-medium text-muted-foreground">
          <span>{t("certificates.columns.subject")}</span>
          <span>{t("certificates.columns.issuer")}</span>
          <span>{t("certificates.columns.expires")}</span>
          <span>{t("certificates.columns.usage")}</span>
          <span />
        </div>
        {certificates.map((cert) => (
          <div
            key={cert.thumbprint}
            className="grid grid-cols-[1.4fr_1.2fr_7rem_1fr_7rem] items-center gap-2 border-b px-3 py-2 text-sm last:border-b-0"
          >
            <span className="truncate" title={cert.subject}>
              {extractCommonName(cert.subject)}
            </span>
            <span className="truncate text-muted-foreground" title={cert.issuer}>
              {extractCommonName(cert.issuer)}
            </span>
            <div className="flex flex-col gap-1">
              <span className="text-muted-foreground">{cert.notAfter}</span>
              {cert.isExpired && (
                <Badge variant="destructive" className="w-fit">
                  {t("certificates.expired")}
                </Badge>
              )}
            </div>
            <span className="truncate text-xs text-muted-foreground">
              {cert.enhancedKeyUsages.length
                ? cert.enhancedKeyUsages.join(", ")
                : t("certificates.anyPurpose")}
            </span>
            <div className="flex items-center justify-end gap-1">
              <DetailsDialog cert={cert} t={t} />
              <ExportButton cert={cert} scope={scope} store={store} t={t} />
              <DeleteDialog
                cert={cert}
                scope={scope}
                store={store}
                isAdministrator={isAdministrator}
                onDeleted={onChanged}
                t={t}
              />
            </div>
          </div>
        ))}
      </div>
    </div>
  );
}

export function CertificatesPage() {
  const { t } = useTranslation();
  const [selected, setSelected] = useState(CERT_STORE_OPTIONS[0]);
  const certs = useCertificates(selected.scope, selected.store);
  const { isAdministrator } = useIsAdministrator();

  const currentUserOptions = CERT_STORE_OPTIONS.filter((o) => o.scope === "CurrentUser");
  const localMachineOptions = CERT_STORE_OPTIONS.filter((o) => o.scope === "LocalMachine");

  return (
    <div className="flex flex-col gap-6">
      <div>
        <h1 className="text-lg font-semibold">{t("certificates.title")}</h1>
        <p className="text-sm text-muted-foreground">{t("certificates.subtitle")}</p>
      </div>

      <Card>
        <CardContent className="flex flex-wrap items-center justify-between gap-3">
          <div className="flex flex-col gap-1.5">
            <Label>{t("certificates.storeLabel")}</Label>
            <Select
              value={storeOptionKey(selected)}
              onValueChange={(value) => {
                const next = CERT_STORE_OPTIONS.find((o) => storeOptionKey(o) === value);
                if (next) setSelected(next);
              }}
            >
              <SelectTrigger className="w-72">
                <SelectValue />
              </SelectTrigger>
              <SelectContent>
                <SelectGroup>
                  <SelectLabel>{t("common.currentUser")}</SelectLabel>
                  {currentUserOptions.map((option) => (
                    <SelectItem key={storeOptionKey(option)} value={storeOptionKey(option)}>
                      {t(option.labelKey)}
                    </SelectItem>
                  ))}
                </SelectGroup>
                <SelectGroup>
                  <SelectLabel>{t("common.localMachine")}</SelectLabel>
                  {localMachineOptions.map((option) => (
                    <SelectItem key={storeOptionKey(option)} value={storeOptionKey(option)}>
                      {t(option.labelKey)}
                    </SelectItem>
                  ))}
                </SelectGroup>
              </SelectContent>
            </Select>
          </div>
          <Button
            variant="ghost"
            size="icon-sm"
            onClick={certs.refresh}
            disabled={certs.status === "loading"}
          >
            <RefreshCw className={certs.status === "loading" ? "animate-spin" : ""} />
          </Button>
        </CardContent>
      </Card>

      {certs.status === "loading" && certs.certificates.length === 0 && (
        <p className="flex items-center gap-2 text-sm text-muted-foreground">
          <Loader2 className="size-4 animate-spin" />
          {t("certificates.reading")}
        </p>
      )}
      {certs.status === "error" && certs.certificates.length === 0 && (
        <p className="text-sm text-destructive">
          {t("certificates.couldntRead", { error: certs.error })}
        </p>
      )}
      {certs.status === "ready" && certs.certificates.length === 0 && (
        <p className="text-sm text-muted-foreground">{t("certificates.noneConfigured")}</p>
      )}
      {certs.certificates.length > 0 && (
        <CertificateTable
          certificates={certs.certificates}
          scope={selected.scope}
          store={selected.store}
          isAdministrator={isAdministrator}
          onChanged={certs.refresh}
          t={t}
        />
      )}
    </div>
  );
}
