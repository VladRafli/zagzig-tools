import { useMemo, useState } from "react";
import { ChevronRight, Loader2, RefreshCw, Search } from "lucide-react";
import { useTranslation } from "react-i18next";
import type { TFunction } from "i18next";

import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { DetailList, DetailRow } from "@/components/detail-list";
import {
  usePorts,
  type PortEntry,
  type PortProcess,
} from "@/features/ports/use-ports";
import { formatRelativeTime } from "@/lib/relative-time";

type StateFilter = "listening" | "established" | "all";
type ProtocolFilter = "all" | "TCP" | "UDP";

const STATE_FILTERS: StateFilter[] = ["listening", "established", "all"];
const PROTOCOL_FILTERS: ProtocolFilter[] = ["all", "TCP", "UDP"];

// UDP endpoints have no state, so they count as "listening" — they're bound
// and waiting for traffic, which is what that filter is for.
function matchesState(entry: PortEntry, filter: StateFilter): boolean {
  if (filter === "all") return true;
  if (filter === "listening") return entry.protocol === "UDP" || entry.state === "Listen";
  return entry.state === "Established";
}

function endpoint(address: string, port: number): string {
  if (!address) return "";
  return address.includes(":") ? `[${address}]:${port}` : `${address}:${port}`;
}

function formatStart(iso: string | null): string | null {
  if (!iso) return null;
  const date = new Date(iso);
  return Number.isNaN(date.getTime()) ? iso : date.toLocaleString();
}

function ProcessDetails({
  process,
  t,
}: {
  process: PortProcess | undefined;
  t: TFunction;
}) {
  if (!process) {
    return (
      <p className="text-sm text-muted-foreground">{t("ports.noProcessInfo")}</p>
    );
  }
  const restricted = !process.path && process.pid > 4;
  return (
    <div className="flex flex-col gap-3">
      <DetailList>
        <DetailRow label={t("ports.details.name")} value={process.name} />
        <DetailRow label={t("ports.details.pid")} value={process.pid} />
        <DetailRow label={t("ports.details.description")} value={process.description} />
        <DetailRow label={t("ports.details.company")} value={process.company} />
        <DetailRow label={t("ports.details.version")} value={process.version} />
        <DetailRow
          label={t("ports.details.started")}
          value={formatStart(process.startTime)}
        />
        <DetailRow
          label={t("ports.details.parent")}
          value={
            process.parentPid
              ? `${process.parentName ?? t("ports.unknown")} (${process.parentPid})`
              : null
          }
        />
        <DetailRow
          label={t("ports.details.memory")}
          value={process.workingSetMb !== null ? `${process.workingSetMb} MB` : null}
        />
        <DetailRow label={t("ports.details.services")} value={process.services} />
      </DetailList>
      <div className="flex flex-col gap-1 text-sm">
        <span className="text-muted-foreground">{t("ports.details.path")}</span>
        <code className="break-all text-xs">
          {process.path ?? t("common.none")}
        </code>
        <span className="mt-1 text-muted-foreground">
          {t("ports.details.commandLine")}
        </span>
        <code className="break-all text-xs">
          {process.commandLine ?? t("common.none")}
        </code>
      </div>
      {restricted && (
        <p className="text-xs text-muted-foreground">{t("ports.restrictedNote")}</p>
      )}
    </div>
  );
}

const GRID = "grid-cols-[1.25rem_4rem_1.2fr_1.2fr_6.5rem_1.2fr]";

function PortRow({
  entry,
  process,
  expanded,
  onToggle,
  t,
}: {
  entry: PortEntry;
  process: PortProcess | undefined;
  expanded: boolean;
  onToggle: () => void;
  t: TFunction;
}) {
  return (
    <div className="border-b last:border-b-0">
      <button
        type="button"
        onClick={onToggle}
        aria-expanded={expanded}
        className={`grid ${GRID} w-full items-center gap-2 px-3 py-2 text-left text-sm hover:bg-muted/40`}
      >
        <ChevronRight
          className={`size-4 text-muted-foreground transition-transform ${
            expanded ? "rotate-90" : ""
          }`}
        />
        <Badge variant={entry.protocol === "TCP" ? "secondary" : "outline"} className="w-fit">
          {entry.protocol}
        </Badge>
        <span className="break-all font-mono text-xs">
          {endpoint(entry.localAddress, entry.localPort)}
        </span>
        <span className="break-all font-mono text-xs text-muted-foreground">
          {entry.remotePort > 0 || (entry.remoteAddress && !/^(0\.0\.0\.0|::)$/.test(entry.remoteAddress))
            ? endpoint(entry.remoteAddress, entry.remotePort)
            : ""}
        </span>
        <span className="text-xs text-muted-foreground">{entry.state}</span>
        <span className="min-w-0 truncate">
          {process?.name ?? t("ports.unknown")}
          <span className="ml-1 text-xs text-muted-foreground">({entry.pid})</span>
        </span>
      </button>
      {expanded && (
        <div className="border-t bg-muted/20 px-4 py-3">
          <ProcessDetails process={process} t={t} />
        </div>
      )}
    </div>
  );
}

export function PortsPage() {
  const { t } = useTranslation();
  const { entries, processes, status, error, updatedAt, refresh } = usePorts();

  const [query, setQuery] = useState("");
  const [stateFilter, setStateFilter] = useState<StateFilter>("listening");
  const [protocolFilter, setProtocolFilter] = useState<ProtocolFilter>("all");
  const [expanded, setExpanded] = useState<string | null>(null);

  const processByPid = useMemo(
    () => new Map(processes.map((p) => [p.pid, p])),
    [processes],
  );

  const rows = useMemo(() => {
    const needle = query.trim().toLowerCase();
    return entries
      .filter((e) => matchesState(e, stateFilter))
      .filter((e) => protocolFilter === "all" || e.protocol === protocolFilter)
      .filter((e) => {
        if (!needle) return true;
        const p = processByPid.get(e.pid);
        return [
          String(e.localPort),
          String(e.remotePort),
          e.localAddress,
          e.remoteAddress,
          String(e.pid),
          p?.name,
          p?.path,
          p?.commandLine,
          p?.description,
          p?.company,
          ...(p?.services ?? []),
        ].some((field) => field?.toLowerCase().includes(needle));
      })
      .sort(
        (a, b) =>
          a.localPort - b.localPort ||
          a.protocol.localeCompare(b.protocol) ||
          a.localAddress.localeCompare(b.localAddress) ||
          a.remotePort - b.remotePort,
      );
  }, [entries, processByPid, query, stateFilter, protocolFilter]);

  const rowKey = (e: PortEntry) =>
    `${e.protocol}|${e.localAddress}|${e.localPort}|${e.remoteAddress}|${e.remotePort}|${e.pid}`;

  return (
    <div className="flex flex-col gap-6">
      <div>
        <h1 className="text-lg font-semibold">{t("ports.title")}</h1>
        <p className="text-sm text-muted-foreground">{t("ports.subtitle")}</p>
      </div>

      <div className="flex flex-col gap-3">
        <div className="flex flex-wrap items-center gap-2">
          <div className="relative min-w-48 flex-1">
            <Search className="pointer-events-none absolute top-1/2 left-2.5 size-4 -translate-y-1/2 text-muted-foreground" />
            <Input
              className="pl-8"
              placeholder={t("ports.searchPlaceholder")}
              value={query}
              onChange={(e) => setQuery(e.currentTarget.value)}
            />
          </div>
          <Select
            value={stateFilter}
            onValueChange={(v) => setStateFilter(v as StateFilter)}
          >
            <SelectTrigger size="sm" className="w-40" aria-label={t("ports.filters.state")}>
              <SelectValue>
                {(v: StateFilter) => t(`ports.filters.states.${v}`)}
              </SelectValue>
            </SelectTrigger>
            <SelectContent>
              {STATE_FILTERS.map((f) => (
                <SelectItem key={f} value={f}>
                  {t(`ports.filters.states.${f}`)}
                </SelectItem>
              ))}
            </SelectContent>
          </Select>
          <Select
            value={protocolFilter}
            onValueChange={(v) => setProtocolFilter(v as ProtocolFilter)}
          >
            <SelectTrigger size="sm" className="w-32" aria-label={t("ports.filters.protocol")}>
              <SelectValue>
                {(v: ProtocolFilter) =>
                  v === "all" ? t("ports.filters.allProtocols") : v
                }
              </SelectValue>
            </SelectTrigger>
            <SelectContent>
              {PROTOCOL_FILTERS.map((f) => (
                <SelectItem key={f} value={f}>
                  {f === "all" ? t("ports.filters.allProtocols") : f}
                </SelectItem>
              ))}
            </SelectContent>
          </Select>
          <div className="ml-auto flex items-center gap-2">
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
            {t("ports.reading")}
          </p>
        )}
        {status === "error" && entries.length === 0 && (
          <p className="text-sm text-destructive">
            {t("ports.couldntRead", { error })}
          </p>
        )}
        {status === "ready" && rows.length === 0 && (
          <p className="text-sm text-muted-foreground">{t("ports.noMatches")}</p>
        )}
        {rows.length > 0 && (
          <>
            <p className="text-xs text-muted-foreground">
              {t("ports.count", { count: rows.length })}
            </p>
            <div className="overflow-x-auto rounded-lg border">
              <div className="min-w-[46rem]">
                <div
                  className={`grid ${GRID} gap-2 border-b bg-muted/50 px-3 py-2 text-xs font-medium text-muted-foreground`}
                >
                  <span />
                  <span>{t("ports.columns.protocol")}</span>
                  <span>{t("ports.columns.local")}</span>
                  <span>{t("ports.columns.remote")}</span>
                  <span>{t("ports.columns.state")}</span>
                  <span>{t("ports.columns.process")}</span>
                </div>
                {rows.map((entry) => {
                  const key = rowKey(entry);
                  return (
                    <PortRow
                      key={key}
                      entry={entry}
                      process={processByPid.get(entry.pid)}
                      expanded={expanded === key}
                      onToggle={() => setExpanded(expanded === key ? null : key)}
                      t={t}
                    />
                  );
                })}
              </div>
            </div>
          </>
        )}
      </div>
    </div>
  );
}
