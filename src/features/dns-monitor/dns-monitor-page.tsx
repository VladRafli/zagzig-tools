import { useState } from "react";
import { Loader2, Play, Plus, Square, Trash2 } from "lucide-react";
import { useTranslation } from "react-i18next";
import type { TFunction } from "i18next";

import { Button } from "@/components/ui/button";
import {
  Card,
  CardHeader,
  CardTitle,
  CardDescription,
  CardContent,
} from "@/components/ui/card";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Badge } from "@/components/ui/badge";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { CollapsibleDetails } from "@/components/collapsible-details";
import { formatRelativeTime } from "@/lib/relative-time";
import {
  useDnsMonitors,
  dnsMonitorStore,
  type DnsMonitor,
} from "@/features/dns-monitor/use-dns-monitor";

const INTERVAL_OPTIONS = [1, 10, 30, 60, 300, 900, 1800];

function MonitorLog({ monitor, t }: { monitor: DnsMonitor; t: TFunction }) {
  if (monitor.entries.length === 0) {
    return (
      <p className="text-sm text-muted-foreground">{t("dnsMonitor.log.empty")}</p>
    );
  }

  return (
    <div className="flex max-h-72 flex-col gap-2 overflow-y-auto">
      {monitor.entries.map((entry) => (
        <div
          key={entry.id}
          className="flex items-center justify-between gap-3 rounded-lg border px-3 py-2 text-sm"
        >
          <div className="flex flex-col">
            <span className="font-medium">
              {entry.resolved
                ? entry.addresses.join(", ")
                : (entry.error ?? t("dnsMonitor.status.unresolvable"))}
            </span>
            <span className="text-xs text-muted-foreground">
              {formatRelativeTime(t, entry.timestamp)}
              {" · "}
              {t("dnsMonitor.status.queryTime", { ms: entry.queryTimeMs })}
            </span>
          </div>
          <Badge variant={entry.resolved ? "default" : "destructive"}>
            {entry.resolved
              ? t("dnsMonitor.status.resolvable")
              : t("dnsMonitor.status.unresolvable")}
          </Badge>
        </div>
      ))}
    </div>
  );
}

function MonitorCard({ monitor, t }: { monitor: DnsMonitor; t: TFunction }) {
  const latest = monitor.entries[0] ?? null;
  const intervalKey = INTERVAL_OPTIONS.includes(monitor.intervalSeconds)
    ? String(monitor.intervalSeconds)
    : null;

  return (
    <Card>
      <CardHeader>
        <div className="flex flex-wrap items-start justify-between gap-3">
          <div>
            <CardTitle className="flex flex-wrap items-center gap-2">
              {monitor.hostname}
              {monitor.running && (
                <Badge variant="outline" className="font-normal">
                  {t("dnsMonitor.running")}
                </Badge>
              )}
            </CardTitle>
            <CardDescription>
              {monitor.server
                ? t("dnsMonitor.status.via", { server: monitor.server })
                : t("dnsMonitor.status.viaSystemDefault")}
              {" · "}
              {intervalKey
                ? t(`dnsMonitor.interval.${intervalKey}`)
                : t("dnsMonitor.status.queryTime", { ms: monitor.intervalSeconds })}
            </CardDescription>
          </div>
          <div className="flex items-center gap-2">
            {monitor.running ? (
              <Button
                variant="outline"
                size="sm"
                onClick={() => dnsMonitorStore.stop(monitor.id)}
              >
                <Square />
                {t("dnsMonitor.stop")}
              </Button>
            ) : (
              <Button
                variant="outline"
                size="sm"
                onClick={() => dnsMonitorStore.start(monitor.id)}
              >
                <Play />
                {t("dnsMonitor.start")}
              </Button>
            )}
            <Button
              variant="ghost"
              size="icon-sm"
              onClick={() => dnsMonitorStore.removeMonitor(monitor.id)}
            >
              <Trash2 />
            </Button>
          </div>
        </div>
      </CardHeader>
      <CardContent className="flex flex-col gap-3">
        {!latest && !monitor.checking && (
          <p className="text-sm text-muted-foreground">{t("dnsMonitor.status.idle")}</p>
        )}
        {monitor.checking && !latest && (
          <p className="flex items-center gap-2 text-sm text-muted-foreground">
            <Loader2 className="size-4 animate-spin" />
            {t("dnsMonitor.status.checking")}
          </p>
        )}
        {latest && (
          <div className="flex flex-col gap-2">
            <div className="flex flex-wrap items-center gap-2">
              <Badge variant={latest.resolved ? "default" : "destructive"}>
                {latest.resolved
                  ? t("dnsMonitor.status.resolvable")
                  : t("dnsMonitor.status.unresolvable")}
              </Badge>
              {monitor.checking && (
                <Loader2 className="size-4 animate-spin text-muted-foreground" />
              )}
              <span className="text-xs text-muted-foreground">
                {t("dnsMonitor.status.lastChecked", {
                  time: formatRelativeTime(t, latest.timestamp),
                })}
                {" · "}
                {t("dnsMonitor.status.queryTime", { ms: latest.queryTimeMs })}
              </span>
            </div>
            {latest.resolved && latest.addresses.length > 0 && (
              <div className="flex flex-wrap gap-1.5">
                {latest.addresses.map((address) => (
                  <Badge key={address} variant="secondary">
                    {address}
                  </Badge>
                ))}
              </div>
            )}
            {!latest.resolved && latest.error && (
              <p className="text-sm text-destructive">{latest.error}</p>
            )}
          </div>
        )}

        <CollapsibleDetails label={t("dnsMonitor.log.title")}>
          <div className="mt-2 flex flex-col gap-2">
            <div className="flex items-center justify-between">
              <p className="text-xs text-muted-foreground">
                {t("dnsMonitor.log.description")}
              </p>
              {monitor.entries.length > 0 && (
                <Button
                  variant="ghost"
                  size="sm"
                  onClick={() => dnsMonitorStore.clearEntries(monitor.id)}
                >
                  <Trash2 />
                  {t("dnsMonitor.log.clear")}
                </Button>
              )}
            </div>
            <MonitorLog monitor={monitor} t={t} />
          </div>
        </CollapsibleDetails>
      </CardContent>
    </Card>
  );
}

export function DnsMonitorPage() {
  const { t } = useTranslation();
  const monitors = useDnsMonitors();
  const [hostname, setHostname] = useState("");
  const [server, setServer] = useState("");
  const [intervalSeconds, setIntervalSeconds] = useState(60);
  const intervalItems = Object.fromEntries(
    INTERVAL_OPTIONS.map((seconds) => [
      String(seconds),
      t(`dnsMonitor.interval.${seconds}`),
    ]),
  );

  function addMonitor() {
    const host = hostname.trim();
    if (!host) return;
    dnsMonitorStore.addMonitor(host, server.trim(), intervalSeconds);
    setHostname("");
    setServer("");
  }

  return (
    <div className="flex flex-col gap-6">
      <div>
        <h1 className="text-lg font-semibold">{t("dnsMonitor.title")}</h1>
        <p className="text-sm text-muted-foreground">{t("dnsMonitor.subtitle")}</p>
      </div>

      <Card>
        <CardContent className="flex flex-col gap-4">
          <div className="grid gap-4 sm:grid-cols-2">
            <div className="flex flex-col gap-1.5">
              <Label htmlFor="hostname">{t("dnsMonitor.hostnameLabel")}</Label>
              <Input
                id="hostname"
                placeholder={t("dnsMonitor.hostnamePlaceholder")}
                value={hostname}
                onChange={(e) => setHostname(e.currentTarget.value)}
                onKeyDown={(e) => {
                  if (e.key === "Enter") addMonitor();
                }}
              />
            </div>
            <div className="flex flex-col gap-1.5">
              <Label htmlFor="server">{t("dnsMonitor.serverLabel")}</Label>
              <Input
                id="server"
                placeholder={t("dnsMonitor.serverPlaceholder")}
                value={server}
                onChange={(e) => setServer(e.currentTarget.value)}
                onKeyDown={(e) => {
                  if (e.key === "Enter") addMonitor();
                }}
              />
            </div>
            <div className="flex flex-col gap-1.5">
              <Label>{t("dnsMonitor.intervalLabel")}</Label>
              <Select
                value={String(intervalSeconds)}
                onValueChange={(value) => setIntervalSeconds(Number(value))}
                items={intervalItems}
              >
                <SelectTrigger className="w-full">
                  <SelectValue />
                </SelectTrigger>
                <SelectContent>
                  {INTERVAL_OPTIONS.map((seconds) => (
                    <SelectItem key={seconds} value={String(seconds)}>
                      {t(`dnsMonitor.interval.${seconds}`)}
                    </SelectItem>
                  ))}
                </SelectContent>
              </Select>
            </div>
          </div>
          <div>
            <Button onClick={addMonitor} disabled={hostname.trim().length === 0}>
              <Plus />
              {t("dnsMonitor.addMonitor")}
            </Button>
          </div>
        </CardContent>
      </Card>

      {monitors.length === 0 ? (
        <p className="text-sm text-muted-foreground">
          {t("dnsMonitor.noMonitorsYet")}
        </p>
      ) : (
        <div className="grid gap-4 lg:grid-cols-2">
          {monitors.map((monitor) => (
            <MonitorCard key={monitor.id} monitor={monitor} t={t} />
          ))}
        </div>
      )}
    </div>
  );
}
