import { useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { Loader2, Plug, PlugZap, RefreshCw } from "lucide-react";
import { useTranslation } from "react-i18next";
import type { TFunction } from "i18next";
import { toast } from "sonner";

import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { DetailList, DetailRow } from "@/components/detail-list";
import { useCachedInvoke } from "@/lib/use-cached-invoke";
import { formatRelativeTime } from "@/lib/relative-time";

interface VpnProfile {
  name: string;
  serverAddress: string;
  status: string;
  tunnelType: string;
  authMethods: string[];
  splitTunneling: boolean;
  rememberCredential: boolean;
  allUsers: boolean;
}

function VpnCard({
  vpn,
  t,
  onChanged,
}: {
  vpn: VpnProfile;
  t: TFunction;
  onChanged: () => void;
}) {
  const [busy, setBusy] = useState(false);
  const connected = vpn.status === "Connected";
  const connecting = vpn.status === "Connecting";

  async function run(action: "connect" | "disconnect") {
    setBusy(true);
    try {
      await invoke("vpn_action", { name: vpn.name, action });
      toast.success(t(action === "connect" ? "vpn.connectSuccess" : "vpn.disconnectSuccess", { name: vpn.name }));
      onChanged();
    } catch (err) {
      toast.error(t("vpn.actionError", { error: err instanceof Error ? err.message : String(err) }));
    } finally {
      setBusy(false);
    }
  }

  return (
    <Card>
      <CardHeader>
        <div className="flex flex-wrap items-center gap-2">
          <CardTitle className="break-all">{vpn.name}</CardTitle>
          <Badge variant={connected ? "default" : "outline"}>
            {connected ? t("vpn.connected") : connecting ? t("vpn.connecting") : t("vpn.disconnected")}
          </Badge>
          {vpn.allUsers && <Badge variant="secondary">{t("vpn.allUsers")}</Badge>}
          <div className="ml-auto">
            {connected ? (
              <Button variant="outline" size="sm" onClick={() => run("disconnect")} disabled={busy}>
                {busy ? <Loader2 className="animate-spin" /> : <Plug />}
                {t("vpn.disconnect")}
              </Button>
            ) : (
              <Button size="sm" onClick={() => run("connect")} disabled={busy || connecting}>
                {busy ? <Loader2 className="animate-spin" /> : <PlugZap />}
                {busy ? t("vpn.connectingNow") : t("vpn.connect")}
              </Button>
            )}
          </div>
        </div>
      </CardHeader>
      <CardContent>
        <DetailList>
          <DetailRow label={t("vpn.details.server")} value={vpn.serverAddress} />
          <DetailRow label={t("vpn.details.tunnel")} value={vpn.tunnelType} />
          <DetailRow label={t("vpn.details.auth")} value={vpn.authMethods} />
          <DetailRow label={t("vpn.details.splitTunneling")} value={vpn.splitTunneling} />
          <DetailRow label={t("vpn.details.rememberCredential")} value={vpn.rememberCredential} />
        </DetailList>
      </CardContent>
    </Card>
  );
}

export function VpnPage() {
  const { t } = useTranslation();
  const { data, status, error, updatedAt, refresh } = useCachedInvoke<VpnProfile[]>(
    "zagzig:vpn",
    "get_vpn_connections",
    15 * 1000,
  );
  const profiles = data ?? [];

  return (
    <div className="flex flex-col gap-6">
      <div>
        <h1 className="text-lg font-semibold">{t("vpn.title")}</h1>
        <p className="text-sm text-muted-foreground">{t("vpn.subtitle")}</p>
      </div>

      <div className="flex flex-col gap-3">
        <div className="flex items-center justify-end gap-2">
          {updatedAt && (
            <span className="text-xs text-muted-foreground">
              {t("common.updatedAgo", { time: formatRelativeTime(t, updatedAt) })}
            </span>
          )}
          <Button variant="ghost" size="icon-sm" onClick={refresh} disabled={status === "loading"}>
            <RefreshCw className={status === "loading" ? "animate-spin" : ""} />
          </Button>
        </div>

        {status === "loading" && profiles.length === 0 && (
          <p className="flex items-center gap-2 text-sm text-muted-foreground">
            <Loader2 className="size-4 animate-spin" />
            {t("vpn.reading")}
          </p>
        )}
        {status === "error" && profiles.length === 0 && (
          <p className="text-sm text-destructive">{t("vpn.couldntRead", { error })}</p>
        )}
        {status === "ready" && profiles.length === 0 && (
          <div className="rounded-lg border p-4 text-sm text-muted-foreground">
            <p>{t("vpn.none")}</p>
            <p className="mt-2">{t("vpn.noneHint")}</p>
          </div>
        )}
        <div className="flex flex-col gap-4">
          {profiles.map((vpn) => (
            <VpnCard key={`${vpn.allUsers}|${vpn.name}`} vpn={vpn} t={t} onChanged={refresh} />
          ))}
        </div>
      </div>
    </div>
  );
}
