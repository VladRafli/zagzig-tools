import { useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { Eraser, Loader2, RefreshCw } from "lucide-react";
import { Trans, useTranslation } from "react-i18next";
import type { TFunction } from "i18next";
import { toast } from "sonner";

import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { useDnsCache, type DnsCacheEntry } from "@/features/dns-cache/use-dns-cache";
import { formatRelativeTime } from "@/lib/relative-time";

function formatTtl(seconds: number): string {
  if (seconds <= 0) return "0s";
  const hours = Math.floor(seconds / 3600);
  const minutes = Math.floor((seconds % 3600) / 60);
  const secs = seconds % 60;
  const parts: string[] = [];
  if (hours) parts.push(`${hours}h`);
  if (minutes) parts.push(`${minutes}m`);
  if (secs || parts.length === 0) parts.push(`${secs}s`);
  return parts.join(" ");
}

function CacheTable({
  entries,
  t,
}: {
  entries: DnsCacheEntry[];
  t: TFunction;
}) {
  return (
    <div className="overflow-x-auto rounded-lg border">
      <div className="min-w-[42rem]">
        <div className="grid grid-cols-[1.4fr_5rem_1.4fr_5rem_9rem] gap-2 border-b bg-muted/50 px-3 py-2 text-xs font-medium text-muted-foreground">
          <span>{t("dnsCache.columns.name")}</span>
          <span>{t("dnsCache.columns.type")}</span>
          <span>{t("dnsCache.columns.data")}</span>
          <span className="text-right">{t("dnsCache.columns.ttl")}</span>
          <span>{t("dnsCache.columns.status")}</span>
        </div>
        {entries.map((entry, index) => (
          <div
            key={`${entry.name}|${entry.recordType}|${entry.data ?? ""}|${index}`}
            className="grid grid-cols-[1.4fr_5rem_1.4fr_5rem_9rem] items-center gap-2 border-b px-3 py-2 text-sm last:border-b-0"
          >
            <span className="break-all">{entry.name}</span>
            <Badge variant="secondary" className="w-fit">
              {entry.recordType}
            </Badge>
            <span className="break-all text-muted-foreground">
              {entry.data ?? t("common.none")}
            </span>
            <span className="text-right text-muted-foreground">
              {formatTtl(entry.timeToLive)}
            </span>
            <span>
              {entry.status && (
                <Badge variant="destructive" className="w-fit">
                  {entry.status}
                </Badge>
              )}
            </span>
          </div>
        ))}
      </div>
    </div>
  );
}

function FlushButton({ t, onFlushed }: { t: TFunction; onFlushed: () => void }) {
  const [flushing, setFlushing] = useState(false);

  async function flush() {
    setFlushing(true);
    try {
      await invoke("flush_dns_cache");
      toast.success(t("dnsCache.flushSuccess"));
      onFlushed();
    } catch (err) {
      const message = err instanceof Error ? err.message : String(err);
      toast.error(t("dnsCache.flushError", { error: message }));
    } finally {
      setFlushing(false);
    }
  }

  return (
    <Button variant="outline" onClick={flush} disabled={flushing}>
      {flushing ? <Loader2 className="animate-spin" /> : <Eraser />}
      {flushing ? t("dnsCache.flushing") : t("dnsCache.flush")}
    </Button>
  );
}

export function DnsCachePage() {
  const { t } = useTranslation();
  const { entries, status, error, updatedAt, refresh } = useDnsCache();

  return (
    <div className="flex flex-col gap-6">
      <div>
        <h1 className="text-lg font-semibold">{t("dnsCache.title")}</h1>
        <p className="text-sm text-muted-foreground">
          <Trans
            i18nKey="dnsCache.subtitle"
            components={{ 1: <code className="text-xs" /> }}
          />
        </p>
      </div>

      <div className="flex flex-col gap-3">
        <div className="flex items-center justify-between">
          <h2 className="text-sm font-medium text-muted-foreground">
            {t("dnsCache.title")}
          </h2>
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

        {status === "loading" && entries.length === 0 && (
          <p className="flex items-center gap-2 text-sm text-muted-foreground">
            <Loader2 className="size-4 animate-spin" />
            {t("dnsCache.reading")}
          </p>
        )}
        {status === "error" && entries.length === 0 && (
          <p className="text-sm text-destructive">
            {t("dnsCache.couldntRead", { error })}
          </p>
        )}
        {status === "ready" && entries.length === 0 && (
          <p className="text-sm text-muted-foreground">
            {t("dnsCache.noneConfigured")}
          </p>
        )}
        {entries.length > 0 && <CacheTable entries={entries} t={t} />}
      </div>

      <div>
        <FlushButton t={t} onFlushed={refresh} />
      </div>
    </div>
  );
}
