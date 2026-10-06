import { useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { Loader2, Lock, Power, RefreshCw, RotateCw } from "lucide-react";
import { useTranslation } from "react-i18next";
import type { TFunction } from "i18next";
import { toast } from "sonner";

import { AdminRequiredTooltip } from "@/components/admin-required-tooltip";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Checkbox } from "@/components/ui/checkbox";
import { Label } from "@/components/ui/label";
import { DetailList, DetailRow } from "@/components/detail-list";
import { useAdapters, type NetworkAdapter } from "@/features/adapters/use-adapters";
import { formatRelativeTime } from "@/lib/relative-time";
import { useIsAdministrator } from "@/lib/use-is-administrator";

function formatBytes(bytes: number): string {
  if (!bytes) return "0 B";
  const units = ["B", "KB", "MB", "GB", "TB"];
  const i = Math.min(Math.floor(Math.log(bytes) / Math.log(1024)), units.length - 1);
  const value = bytes / 1024 ** i;
  return `${value >= 100 || i === 0 ? Math.round(value) : value.toFixed(1)} ${units[i]}`;
}

function errorMessage(err: unknown) {
  return err instanceof Error ? err.message : String(err);
}

function statusVariant(status: string) {
  if (status === "Up") return "default" as const;
  if (status === "Disabled") return "outline" as const;
  return "secondary" as const;
}

function AdapterCard({
  adapter,
  t,
  isAdministrator,
  onChanged,
}: {
  adapter: NetworkAdapter;
  t: TFunction;
  isAdministrator: boolean;
  onChanged: () => void;
}) {
  const [busy, setBusy] = useState<"toggle" | "renew" | null>(null);
  const disabled = adapter.status === "Disabled";

  async function run(kind: "toggle" | "renew") {
    setBusy(kind);
    try {
      if (kind === "toggle") {
        await invoke("set_adapter_enabled", {
          interfaceIndex: adapter.interfaceIndex,
          enabled: disabled,
        });
        toast.success(
          t(disabled ? "adapters.enabledSuccess" : "adapters.disabledSuccess", {
            name: adapter.name,
          }),
        );
      } else {
        await invoke("renew_adapter_dhcp", { interfaceIndex: adapter.interfaceIndex });
        toast.success(t("adapters.renewSuccess", { name: adapter.name }));
      }
      onChanged();
    } catch (err) {
      toast.error(t("adapters.actionError", { error: errorMessage(err) }));
    } finally {
      setBusy(null);
    }
  }

  return (
    <Card>
      <CardHeader>
        <div className="flex flex-wrap items-center gap-2">
          <CardTitle className="break-all">{adapter.name}</CardTitle>
          <Badge variant={statusVariant(adapter.status)}>{adapter.status}</Badge>
          {adapter.isVirtual && <Badge variant="outline">{t("adapters.virtual")}</Badge>}
          <div className="ml-auto flex items-center gap-1">
            <AdminRequiredTooltip locked={!isAdministrator}>
              <Button
                variant="outline"
                size="sm"
                disabled={!isAdministrator || busy !== null || !adapter.dhcp || disabled}
                onClick={() => run("renew")}
                title={adapter.dhcp ? undefined : t("adapters.renewNeedsDhcp")}
              >
                {!isAdministrator ? (
                  <Lock />
                ) : busy === "renew" ? (
                  <Loader2 className="animate-spin" />
                ) : (
                  <RotateCw />
                )}
                {t("adapters.renew")}
              </Button>
            </AdminRequiredTooltip>
            <AdminRequiredTooltip locked={!isAdministrator}>
              <Button
                variant={disabled ? "default" : "outline"}
                size="sm"
                disabled={!isAdministrator || busy !== null}
                onClick={() => run("toggle")}
              >
                {!isAdministrator ? (
                  <Lock />
                ) : busy === "toggle" ? (
                  <Loader2 className="animate-spin" />
                ) : (
                  <Power />
                )}
                {disabled ? t("adapters.enable") : t("adapters.disable")}
              </Button>
            </AdminRequiredTooltip>
          </div>
        </div>
        <p className="text-sm text-muted-foreground">{adapter.description}</p>
      </CardHeader>
      <CardContent>
        <DetailList>
          <DetailRow label={t("adapters.details.ipv4")} value={adapter.ipv4} />
          <DetailRow label={t("adapters.details.gateway")} value={adapter.gateways} />
          <DetailRow label={t("adapters.details.dns")} value={adapter.dnsServers} />
          <DetailRow label={t("adapters.details.dhcp")} value={adapter.dhcp} />
          <DetailRow label={t("adapters.details.mac")} value={adapter.macAddress} />
          <DetailRow label={t("adapters.details.linkSpeed")} value={adapter.linkSpeed} />
          <DetailRow label={t("adapters.details.mtu")} value={adapter.mtu} />
          <DetailRow label={t("adapters.details.media")} value={adapter.mediaType} />
          <DetailRow
            label={t("adapters.details.received")}
            value={formatBytes(adapter.bytesReceived)}
          />
          <DetailRow
            label={t("adapters.details.sent")}
            value={formatBytes(adapter.bytesSent)}
          />
          <DetailRow label={t("adapters.details.ipv6")} value={adapter.ipv6} />
        </DetailList>
      </CardContent>
    </Card>
  );
}

export function AdaptersPage() {
  const { t } = useTranslation();
  const { adapters, status, error, updatedAt, refresh } = useAdapters();
  const { isAdministrator } = useIsAdministrator();
  const [showVirtual, setShowVirtual] = useState(true);

  const visible = adapters.filter((a) => showVirtual || !a.isVirtual);

  return (
    <div className="flex flex-col gap-6">
      <div>
        <h1 className="text-lg font-semibold">{t("adapters.title")}</h1>
        <p className="text-sm text-muted-foreground">{t("adapters.subtitle")}</p>
      </div>

      <div className="flex flex-col gap-3">
        <div className="flex items-center justify-between">
          <div className="flex items-center gap-2">
            <Checkbox
              id="adaptersShowVirtual"
              checked={showVirtual}
              onCheckedChange={(checked) => setShowVirtual(checked === true)}
            />
            <Label htmlFor="adaptersShowVirtual">{t("adapters.showVirtual")}</Label>
          </div>
          <div className="flex items-center gap-2">
            {updatedAt && (
              <span className="text-xs text-muted-foreground">
                {t("common.updatedAgo", {
                  time: formatRelativeTime(t, updatedAt),
                })}
              </span>
            )}
            <Button
              variant="ghost"
              size="icon-sm"
              onClick={refresh}
              disabled={status === "loading"}
            >
              <RefreshCw className={status === "loading" ? "animate-spin" : ""} />
            </Button>
          </div>
        </div>

        {status === "loading" && adapters.length === 0 && (
          <p className="flex items-center gap-2 text-sm text-muted-foreground">
            <Loader2 className="size-4 animate-spin" />
            {t("adapters.reading")}
          </p>
        )}
        {status === "error" && adapters.length === 0 && (
          <p className="text-sm text-destructive">
            {t("adapters.couldntRead", { error })}
          </p>
        )}
        {status === "ready" && visible.length === 0 && (
          <p className="text-sm text-muted-foreground">{t("adapters.none")}</p>
        )}
        <div className="flex flex-col gap-4">
          {visible.map((adapter) => (
            <AdapterCard
              key={adapter.interfaceIndex}
              adapter={adapter}
              t={t}
              isAdministrator={isAdministrator}
              onChanged={refresh}
            />
          ))}
        </div>
      </div>
    </div>
  );
}
