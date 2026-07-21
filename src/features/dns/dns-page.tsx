import { useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { Loader2, Lock, Pencil, Plus, RefreshCw, RotateCcw, Trash2 } from "lucide-react";
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
import { Input } from "@/components/ui/input";
import { Badge } from "@/components/ui/badge";
import { useDnsSettings, type DnsInterface } from "@/features/dns/use-dns-settings";
import { formatRelativeTime } from "@/lib/relative-time";
import { useIsAdministrator } from "@/lib/use-is-administrator";
import { AdminRequiredTooltip } from "@/components/admin-required-tooltip";

function EditDnsDialog({
  iface,
  t,
  isAdministrator,
  onSaved,
}: {
  iface: DnsInterface;
  t: TFunction;
  isAdministrator: boolean;
  onSaved: () => void;
}) {
  const [open, setOpen] = useState(false);
  const [servers, setServers] = useState<string[]>(
    iface.serverAddresses.length ? iface.serverAddresses : [""],
  );
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState<string | null>(null);

  function handleOpenChange(next: boolean) {
    if (saving) return;
    setOpen(next);
    if (next) {
      setServers(iface.serverAddresses.length ? iface.serverAddresses : [""]);
      setError(null);
    }
  }

  function updateServer(index: number, value: string) {
    setServers((prev) => prev.map((s, i) => (i === index ? value : s)));
  }

  function addServer() {
    setServers((prev) => [...prev, ""]);
  }

  function removeServer(index: number) {
    setServers((prev) => prev.filter((_, i) => i !== index));
  }

  async function save() {
    const cleaned = servers.map((s) => s.trim()).filter(Boolean);
    if (cleaned.length === 0) {
      setError(t("dns.saveError", { error: "Enter at least one DNS server." }));
      return;
    }
    setSaving(true);
    setError(null);
    try {
      await invoke("set_dns_servers", {
        interfaceIndex: iface.interfaceIndex,
        servers: cleaned,
      });
      setOpen(false);
      toast.success(t("dns.saveSuccess", { interface: iface.interfaceAlias }));
      onSaved();
    } catch (err) {
      const message = err instanceof Error ? err.message : String(err);
      setError(t("dns.saveError", { error: message }));
    } finally {
      setSaving(false);
    }
  }

  return (
    <Dialog open={open} onOpenChange={handleOpenChange}>
      <AdminRequiredTooltip locked={!isAdministrator}>
        <DialogTrigger
          render={
            <Button variant="outline" size="sm" disabled={!isAdministrator} />
          }
        >
          {isAdministrator ? <Pencil /> : <Lock />}
          {t("dns.edit")}
        </DialogTrigger>
      </AdminRequiredTooltip>
      <DialogContent>
        <DialogHeader>
          <DialogTitle>
            {t("dns.editTitle", { interface: iface.interfaceAlias })}
          </DialogTitle>
          <DialogDescription>{t("dns.editDescription")}</DialogDescription>
        </DialogHeader>

        <div className="flex flex-col gap-2">
          {servers.map((server, index) => (
            <div key={index} className="flex items-center gap-2">
              <Input
                value={server}
                placeholder={t("dns.serverPlaceholder")}
                onChange={(e) => updateServer(index, e.currentTarget.value)}
              />
              <Button
                variant="ghost"
                size="icon-sm"
                onClick={() => removeServer(index)}
                disabled={servers.length === 1}
              >
                <Trash2 />
              </Button>
            </div>
          ))}
          <Button
            variant="outline"
            size="sm"
            className="w-fit"
            onClick={addServer}
          >
            <Plus />
            {t("dns.addServer")}
          </Button>
        </div>

        {error && <p className="text-sm text-destructive">{error}</p>}

        <DialogFooter>
          <Button
            variant="outline"
            onClick={() => handleOpenChange(false)}
            disabled={saving}
          >
            {t("dns.cancel")}
          </Button>
          <Button onClick={save} disabled={saving}>
            {saving && <Loader2 className="animate-spin" />}
            {saving ? t("dns.saving") : t("dns.save")}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}

function ResetDnsDialog({
  iface,
  t,
  isAdministrator,
  onReset,
}: {
  iface: DnsInterface;
  t: TFunction;
  isAdministrator: boolean;
  onReset: () => void;
}) {
  const [open, setOpen] = useState(false);
  const [resetting, setResetting] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const disabled = !isAdministrator || iface.serverAddresses.length === 0;

  function handleOpenChange(next: boolean) {
    if (resetting) return;
    setOpen(next);
    if (!next) setError(null);
  }

  async function confirmReset() {
    setResetting(true);
    setError(null);
    try {
      await invoke("reset_dns_servers", {
        interfaceIndex: iface.interfaceIndex,
      });
      setOpen(false);
      toast.success(t("dns.resetSuccess", { interface: iface.interfaceAlias }));
      onReset();
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setResetting(false);
    }
  }

  return (
    <Dialog open={open} onOpenChange={handleOpenChange}>
      <AdminRequiredTooltip locked={!isAdministrator}>
        <DialogTrigger
          render={<Button variant="ghost" size="sm" disabled={disabled} />}
        >
          {!isAdministrator ? <Lock /> : <RotateCcw />}
          {t("dns.reset")}
        </DialogTrigger>
      </AdminRequiredTooltip>
      <DialogContent>
        <DialogHeader>
          <DialogTitle>{t("dns.resetConfirmTitle")}</DialogTitle>
          <DialogDescription>
            {t("dns.resetConfirmDescription", {
              interface: iface.interfaceAlias,
            })}
          </DialogDescription>
        </DialogHeader>
        {error && <p className="text-sm text-destructive">{error}</p>}
        <DialogFooter>
          <Button
            variant="outline"
            onClick={() => handleOpenChange(false)}
            disabled={resetting}
          >
            {t("dns.cancel")}
          </Button>
          <Button
            variant="destructive"
            onClick={confirmReset}
            disabled={resetting}
          >
            {resetting && <Loader2 className="animate-spin" />}
            {resetting ? t("dns.resetting") : t("dns.reset")}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}

function DnsInterfaceCard({
  iface,
  t,
  isAdministrator,
  onChanged,
}: {
  iface: DnsInterface;
  t: TFunction;
  isAdministrator: boolean;
  onChanged: () => void;
}) {
  return (
    <Card>
      <CardContent className="flex flex-col gap-3">
        <div className="flex flex-wrap items-center justify-between gap-2">
          <div className="flex flex-wrap items-center gap-1.5">
            <span className="font-medium">{iface.interfaceAlias}</span>
            <Badge variant={iface.status === "Up" ? "default" : "secondary"}>
              {iface.status}
            </Badge>
            <Badge variant="outline">
              {iface.dhcp ? t("dns.automatic") : t("dns.manual")}
            </Badge>
          </div>
          <div className="flex items-center gap-2">
            <ResetDnsDialog
              iface={iface}
              t={t}
              isAdministrator={isAdministrator}
              onReset={onChanged}
            />
            <EditDnsDialog
              iface={iface}
              t={t}
              isAdministrator={isAdministrator}
              onSaved={onChanged}
            />
          </div>
        </div>

        <div className="flex flex-wrap gap-1.5">
          {iface.serverAddresses.length === 0 ? (
            <span className="text-sm text-muted-foreground">
              {t("dns.noServersSet")}
            </span>
          ) : (
            iface.serverAddresses.map((server) => (
              <Badge key={server} variant="secondary">
                {server}
              </Badge>
            ))
          )}
        </div>
      </CardContent>
    </Card>
  );
}

export function DnsPage() {
  const { t } = useTranslation();
  const dns = useDnsSettings();
  const { isAdministrator } = useIsAdministrator();

  return (
    <div className="flex flex-col gap-6">
      <div>
        <h1 className="text-lg font-semibold">{t("dns.title")}</h1>
        <p className="text-sm text-muted-foreground">{t("dns.subtitle")}</p>
      </div>

      <div className="flex flex-col gap-3">
        <div className="flex items-center justify-between">
          <div className="flex items-center gap-2">
            {dns.updatedAt && (
              <span className="text-xs text-muted-foreground">
                {t("common.updatedAgo", {
                  time: formatRelativeTime(t, dns.updatedAt),
                })}
              </span>
            )}
          </div>
          <Button
            variant="ghost"
            size="icon-sm"
            onClick={dns.refresh}
            disabled={dns.status === "loading"}
          >
            <RefreshCw
              className={dns.status === "loading" ? "animate-spin" : ""}
            />
          </Button>
        </div>

        {dns.status === "loading" && dns.interfaces.length === 0 && (
          <p className="flex items-center gap-2 text-sm text-muted-foreground">
            <Loader2 className="size-4 animate-spin" />
            {t("dns.reading")}
          </p>
        )}
        {dns.status === "error" && dns.interfaces.length === 0 && (
          <p className="text-sm text-destructive">
            {t("dns.couldntRead", { error: dns.error })}
          </p>
        )}
        {dns.status === "ready" && dns.interfaces.length === 0 && (
          <p className="text-sm text-muted-foreground">
            {t("dns.noneConfigured")}
          </p>
        )}
        {dns.interfaces.map((iface) => (
          <DnsInterfaceCard
            key={iface.interfaceIndex}
            iface={iface}
            t={t}
            isAdministrator={isAdministrator}
            onChanged={dns.refresh}
          />
        ))}
      </div>
    </div>
  );
}
