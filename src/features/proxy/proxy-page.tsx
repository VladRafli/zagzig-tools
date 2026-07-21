import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { Download, Loader2, Lock, RefreshCw, RotateCcw } from "lucide-react";
import { useTranslation } from "react-i18next";
import type { TFunction } from "i18next";
import { toast } from "sonner";

import { Button } from "@/components/ui/button";
import {
  Card,
  CardHeader,
  CardTitle,
  CardDescription,
  CardContent,
} from "@/components/ui/card";
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
import { Label } from "@/components/ui/label";
import { Badge } from "@/components/ui/badge";
import { useWinHttpProxy } from "@/features/proxy/use-winhttp-proxy";
import { formatRelativeTime } from "@/lib/relative-time";
import { useIsAdministrator } from "@/lib/use-is-administrator";
import { AdminRequiredTooltip } from "@/components/admin-required-tooltip";

function CurrentProxyCard({
  proxy,
  status,
  error,
  updatedAt,
  onRefresh,
  t,
}: {
  proxy: ReturnType<typeof useWinHttpProxy>["proxy"];
  status: "loading" | "ready" | "error";
  error: string | null;
  updatedAt: number | null;
  onRefresh: () => void;
  t: TFunction;
}) {
  return (
    <Card>
      <CardHeader>
        <div className="flex items-center justify-between">
          <div>
            <CardTitle>{t("proxy.currentTitle")}</CardTitle>
            <CardDescription>{t("proxy.currentDescription")}</CardDescription>
          </div>
          <Button variant="ghost" size="icon-sm" onClick={onRefresh} disabled={status === "loading"}>
            <RefreshCw className={status === "loading" ? "animate-spin" : ""} />
          </Button>
        </div>
      </CardHeader>
      <CardContent>
        {status === "loading" && !proxy && (
          <p className="flex items-center gap-2 text-sm text-muted-foreground">
            <Loader2 className="size-4 animate-spin" />
            {t("proxy.reading")}
          </p>
        )}
        {status === "error" && !proxy && (
          <p className="text-sm text-destructive">
            {t("proxy.couldntRead", { error })}
          </p>
        )}
        {proxy && (
          <div className="flex flex-col gap-2">
            <div className="flex items-center gap-2">
              <Badge variant={proxy.enabled ? "default" : "secondary"}>
                {proxy.enabled ? t("proxy.proxyServer") : t("proxy.directAccess")}
              </Badge>
              {updatedAt && (
                <span className="text-xs text-muted-foreground">
                  {t("common.updatedAgo", { time: formatRelativeTime(t, updatedAt) })}
                </span>
              )}
            </div>
            {proxy.enabled && (
              <div className="flex flex-col gap-1 text-sm">
                <span className="font-mono">{proxy.proxyServer}</span>
                {proxy.bypassList && (
                  <span className="text-xs text-muted-foreground">
                    {t("proxy.bypassList")}: {proxy.bypassList}
                  </span>
                )}
              </div>
            )}
          </div>
        )}
      </CardContent>
    </Card>
  );
}

function SetProxyCard({
  proxy,
  isAdministrator,
  onChanged,
  t,
}: {
  proxy: ReturnType<typeof useWinHttpProxy>["proxy"];
  isAdministrator: boolean;
  onChanged: () => void;
  t: TFunction;
}) {
  const [proxyServer, setProxyServer] = useState(proxy?.proxyServer ?? "");
  const [bypassList, setBypassList] = useState(proxy?.bypassList ?? "");
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    setProxyServer(proxy?.proxyServer ?? "");
    setBypassList(proxy?.bypassList ?? "");
  }, [proxy?.proxyServer, proxy?.bypassList]);

  const canSave = isAdministrator && proxyServer.trim().length > 0 && !saving;

  async function save() {
    if (!canSave) return;
    setSaving(true);
    setError(null);
    try {
      await invoke("set_winhttp_proxy", {
        proxyServer: proxyServer.trim(),
        bypassList: bypassList.trim() || undefined,
      });
      toast.success(t("proxy.saveSuccess"));
      onChanged();
    } catch (err) {
      const message = err instanceof Error ? err.message : String(err);
      setError(t("proxy.saveError", { error: message }));
    } finally {
      setSaving(false);
    }
  }

  return (
    <Card>
      <CardHeader>
        <div className="flex items-center gap-2">
          <CardTitle>{t("proxy.editTitle")}</CardTitle>
          {!isAdministrator && (
            <Badge variant="outline" className="gap-1">
              <Lock className="size-3" />
              {t("common.administratorOnly")}
            </Badge>
          )}
        </div>
        <CardDescription>{t("proxy.editDescription")}</CardDescription>
      </CardHeader>
      <CardContent className="flex flex-col gap-4">
        <div className="grid gap-4 sm:grid-cols-2">
          <div className="flex flex-col gap-1.5">
            <Label htmlFor="proxyServer">{t("proxy.proxyServerLabel")}</Label>
            <Input
              id="proxyServer"
              placeholder={t("proxy.proxyServerPlaceholder")}
              value={proxyServer}
              onChange={(e) => setProxyServer(e.currentTarget.value)}
              disabled={!isAdministrator}
            />
          </div>
          <div className="flex flex-col gap-1.5">
            <Label htmlFor="bypassList">{t("proxy.bypassListLabel")}</Label>
            <Input
              id="bypassList"
              placeholder={t("proxy.bypassListPlaceholder")}
              value={bypassList}
              onChange={(e) => setBypassList(e.currentTarget.value)}
              disabled={!isAdministrator}
            />
          </div>
        </div>

        {error && <p className="text-sm text-destructive">{error}</p>}

        <div>
          <AdminRequiredTooltip locked={!isAdministrator}>
            <Button onClick={save} disabled={!canSave}>
              {!isAdministrator ? (
                <Lock />
              ) : saving ? (
                <Loader2 className="animate-spin" />
              ) : null}
              {saving ? t("proxy.saving") : t("proxy.save")}
            </Button>
          </AdminRequiredTooltip>
        </div>
      </CardContent>
    </Card>
  );
}

function ResetProxyDialog({
  isAdministrator,
  onReset,
  t,
}: {
  isAdministrator: boolean;
  onReset: () => void;
  t: TFunction;
}) {
  const [open, setOpen] = useState(false);
  const [resetting, setResetting] = useState(false);
  const [error, setError] = useState<string | null>(null);

  function handleOpenChange(next: boolean) {
    if (resetting) return;
    setOpen(next);
    if (!next) setError(null);
  }

  async function confirmReset() {
    setResetting(true);
    setError(null);
    try {
      await invoke("reset_winhttp_proxy");
      setOpen(false);
      toast.success(t("proxy.resetSuccess"));
      onReset();
    } catch (err) {
      const message = err instanceof Error ? err.message : String(err);
      setError(t("proxy.resetError", { error: message }));
    } finally {
      setResetting(false);
    }
  }

  return (
    <Dialog open={open} onOpenChange={handleOpenChange}>
      <AdminRequiredTooltip locked={!isAdministrator}>
        <DialogTrigger render={<Button variant="outline" disabled={!isAdministrator} />}>
          {isAdministrator ? <RotateCcw /> : <Lock />}
          {t("proxy.reset")}
        </DialogTrigger>
      </AdminRequiredTooltip>
      <DialogContent>
        <DialogHeader>
          <DialogTitle>{t("proxy.reset")}</DialogTitle>
          <DialogDescription>{t("proxy.editDescription")}</DialogDescription>
        </DialogHeader>
        {error && <p className="text-sm text-destructive">{error}</p>}
        <DialogFooter>
          <Button variant="outline" onClick={() => handleOpenChange(false)} disabled={resetting}>
            {t("hosts.cancel")}
          </Button>
          <Button variant="destructive" onClick={confirmReset} disabled={resetting}>
            {resetting && <Loader2 className="animate-spin" />}
            {resetting ? t("proxy.resetting") : t("proxy.reset")}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}

function ImportFromSystemButton({
  isAdministrator,
  onImported,
  t,
}: {
  isAdministrator: boolean;
  onImported: () => void;
  t: TFunction;
}) {
  const [importing, setImporting] = useState(false);

  async function importProxy() {
    setImporting(true);
    try {
      await invoke("import_winhttp_proxy_from_system");
      toast.success(t("proxy.importSuccess"));
      onImported();
    } catch (err) {
      const message = err instanceof Error ? err.message : String(err);
      toast.error(t("proxy.importError", { error: message }));
    } finally {
      setImporting(false);
    }
  }

  return (
    <div className="flex flex-col gap-2">
      <AdminRequiredTooltip locked={!isAdministrator}>
        <Button variant="outline" onClick={importProxy} disabled={!isAdministrator || importing}>
          {importing ? <Loader2 className="animate-spin" /> : isAdministrator ? <Download /> : <Lock />}
          {importing ? t("proxy.importing") : t("proxy.importFromSystem")}
        </Button>
      </AdminRequiredTooltip>
      <p className="text-xs text-muted-foreground">{t("proxy.importHint")}</p>
    </div>
  );
}

export function ProxyPage() {
  const { t } = useTranslation();
  const { proxy, status, error, updatedAt, refresh } = useWinHttpProxy();
  const { isAdministrator } = useIsAdministrator();

  return (
    <div className="flex flex-col gap-6">
      <div>
        <h1 className="text-lg font-semibold">{t("proxy.title")}</h1>
        <p className="text-sm text-muted-foreground">{t("proxy.subtitle")}</p>
      </div>

      <CurrentProxyCard
        proxy={proxy}
        status={status}
        error={error}
        updatedAt={updatedAt}
        onRefresh={refresh}
        t={t}
      />

      <SetProxyCard
        proxy={proxy}
        isAdministrator={isAdministrator}
        onChanged={refresh}
        t={t}
      />

      <div className="flex flex-wrap items-start gap-4">
        <ResetProxyDialog isAdministrator={isAdministrator} onReset={refresh} t={t} />
        <ImportFromSystemButton isAdministrator={isAdministrator} onImported={refresh} t={t} />
      </div>
    </div>
  );
}
