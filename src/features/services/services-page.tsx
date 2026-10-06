import { useMemo, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { Loader2, Lock, Play, RefreshCw, RotateCw, Search, Square } from "lucide-react";
import { useTranslation } from "react-i18next";
import type { TFunction } from "i18next";
import { toast } from "sonner";

import { AdminRequiredTooltip } from "@/components/admin-required-tooltip";
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
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { useCachedInvoke } from "@/lib/use-cached-invoke";
import { formatRelativeTime } from "@/lib/relative-time";
import { useIsAdministrator } from "@/lib/use-is-administrator";

interface WindowsService {
  name: string;
  displayName: string;
  state: string;
  startMode: string;
  delayed: boolean;
  account: string | null;
  path: string | null;
  pid: number;
  description: string | null;
  canStop: boolean;
}

type StateFilter = "all" | "running" | "stopped";
type StartupFilter = "all" | "auto" | "manual" | "disabled";
type StartupType = "auto" | "auto-delayed" | "manual" | "disabled";

const PAGE_SIZE = 100;
const GRID = "grid-cols-[1.6fr_5.5rem_8.5rem_auto]";

// The combined startup value shown for a service ("Auto" with the delayed
// flag becomes its own entry, matching services.msc).
function startupOf(s: WindowsService): StartupType | "other" {
  const mode = s.startMode.toLowerCase();
  if (mode === "auto") return s.delayed ? "auto-delayed" : "auto";
  if (mode === "manual") return "manual";
  if (mode === "disabled") return "disabled";
  return "other"; // boot / system drivers
}

function errorMessage(err: unknown) {
  return err instanceof Error ? err.message : String(err);
}

interface Pending {
  service: WindowsService;
  action: "stop" | "restart" | "disabled";
}

function ServiceRow({
  s,
  t,
  isAdministrator,
  busy,
  onAction,
}: {
  s: WindowsService;
  t: TFunction;
  isAdministrator: boolean;
  busy: boolean;
  onAction: (s: WindowsService, action: string) => void;
}) {
  const running = s.state === "Running";
  const startup = startupOf(s);
  const locked = !isAdministrator;

  return (
    <div className={`grid ${GRID} items-center gap-2 border-b px-3 py-2 text-sm last:border-b-0`}>
      <div className="min-w-0">
        <div className="break-words">{s.displayName}</div>
        <div className="truncate text-xs text-muted-foreground" title={s.description ?? s.name}>
          {s.name}
          {s.description ? ` — ${s.description}` : ""}
        </div>
      </div>
      <Badge variant={running ? "default" : "outline"} className="w-fit">
        {running ? t("services.running") : s.state === "Stopped" ? t("services.stopped") : s.state}
      </Badge>
      <AdminRequiredTooltip locked={locked}>
        <Select
          value={startup === "other" ? undefined : startup}
          onValueChange={(v) => onAction(s, v as string)}
          disabled={locked || busy || startup === "other"}
        >
          <SelectTrigger size="sm" className="w-full" aria-label={t("services.startupType")}>
            <SelectValue placeholder={s.startMode}>
              {(v: StartupType) => t(`services.startup.${v}`)}
            </SelectValue>
          </SelectTrigger>
          <SelectContent>
            {(["auto", "auto-delayed", "manual", "disabled"] as StartupType[]).map((o) => (
              <SelectItem key={o} value={o}>
                {t(`services.startup.${o}`)}
              </SelectItem>
            ))}
          </SelectContent>
        </Select>
      </AdminRequiredTooltip>
      <div className="flex justify-end gap-1">
        <AdminRequiredTooltip locked={locked}>
          {running ? (
            <Button
              variant="ghost"
              size="icon-sm"
              aria-label={t("services.stop")}
              title={t("services.stop")}
              disabled={locked || busy || !s.canStop}
              onClick={() => onAction(s, "stop")}
            >
              {locked ? <Lock /> : busy ? <Loader2 className="animate-spin" /> : <Square />}
            </Button>
          ) : (
            <Button
              variant="ghost"
              size="icon-sm"
              aria-label={t("services.start")}
              title={t("services.start")}
              disabled={locked || busy || s.startMode.toLowerCase() === "disabled"}
              onClick={() => onAction(s, "start")}
            >
              {locked ? <Lock /> : busy ? <Loader2 className="animate-spin" /> : <Play />}
            </Button>
          )}
        </AdminRequiredTooltip>
        <AdminRequiredTooltip locked={locked}>
          <Button
            variant="ghost"
            size="icon-sm"
            aria-label={t("services.restart")}
            title={t("services.restart")}
            disabled={locked || busy || !running || !s.canStop}
            onClick={() => onAction(s, "restart")}
          >
            {locked ? <Lock /> : <RotateCw />}
          </Button>
        </AdminRequiredTooltip>
      </div>
    </div>
  );
}

export function ServicesPage() {
  const { t } = useTranslation();
  const { data, status, error, updatedAt, refresh } = useCachedInvoke<WindowsService[]>(
    "zagzig:services",
    "get_services",
    30 * 1000,
  );
  const { isAdministrator } = useIsAdministrator();

  const [query, setQuery] = useState("");
  const [stateFilter, setStateFilter] = useState<StateFilter>("all");
  const [startupFilter, setStartupFilter] = useState<StartupFilter>("all");
  const [limit, setLimit] = useState(PAGE_SIZE);
  const [busyName, setBusyName] = useState<string | null>(null);
  const [pending, setPending] = useState<Pending | null>(null);

  const services = data ?? [];
  const filtered = useMemo(() => {
    const needle = query.trim().toLowerCase();
    return services.filter((s) => {
      if (stateFilter === "running" && s.state !== "Running") return false;
      if (stateFilter === "stopped" && s.state === "Running") return false;
      const mode = s.startMode.toLowerCase();
      if (startupFilter === "auto" && mode !== "auto") return false;
      if (startupFilter === "manual" && mode !== "manual") return false;
      if (startupFilter === "disabled" && mode !== "disabled") return false;
      return (
        !needle ||
        [s.name, s.displayName, s.description, s.account, s.path].some((f) =>
          f?.toLowerCase().includes(needle),
        )
      );
    });
  }, [services, query, stateFilter, startupFilter]);

  async function perform(service: WindowsService, action: string) {
    setBusyName(service.name);
    try {
      await invoke("service_action", { name: service.name, action });
      toast.success(t("services.actionSuccess", { name: service.displayName }));
      refresh();
    } catch (err) {
      toast.error(t("services.actionError", { error: errorMessage(err) }));
    } finally {
      setBusyName(null);
    }
  }

  // Stopping, restarting or disabling can take a dependent service or the
  // network down with it, so those ask first; the rest just run.
  function handleAction(service: WindowsService, action: string) {
    if (action === "stop" || action === "restart" || action === "disabled") {
      setPending({ service, action });
    } else {
      void perform(service, action);
    }
  }

  return (
    <div className="flex flex-col gap-6">
      <div>
        <h1 className="text-lg font-semibold">{t("services.title")}</h1>
        <p className="text-sm text-muted-foreground">{t("services.subtitle")}</p>
      </div>

      <div className="flex flex-col gap-3">
        <div className="flex flex-wrap items-center gap-2">
          <div className="relative min-w-48 flex-1">
            <Search className="pointer-events-none absolute top-1/2 left-2.5 size-4 -translate-y-1/2 text-muted-foreground" />
            <Input
              className="pl-8"
              placeholder={t("services.searchPlaceholder")}
              value={query}
              onChange={(e) => {
                setQuery(e.currentTarget.value);
                setLimit(PAGE_SIZE);
              }}
            />
          </div>
          <Select
            value={stateFilter}
            onValueChange={(v) => {
              setStateFilter(v as StateFilter);
              setLimit(PAGE_SIZE);
            }}
          >
            <SelectTrigger size="sm" className="w-36" aria-label={t("services.stateFilter")}>
              <SelectValue>{(v: StateFilter) => t(`services.stateFilters.${v}`)}</SelectValue>
            </SelectTrigger>
            <SelectContent>
              {(["all", "running", "stopped"] as StateFilter[]).map((f) => (
                <SelectItem key={f} value={f}>
                  {t(`services.stateFilters.${f}`)}
                </SelectItem>
              ))}
            </SelectContent>
          </Select>
          <Select
            value={startupFilter}
            onValueChange={(v) => {
              setStartupFilter(v as StartupFilter);
              setLimit(PAGE_SIZE);
            }}
          >
            <SelectTrigger size="sm" className="w-40" aria-label={t("services.startupFilter")}>
              <SelectValue>{(v: StartupFilter) => t(`services.startupFilters.${v}`)}</SelectValue>
            </SelectTrigger>
            <SelectContent>
              {(["all", "auto", "manual", "disabled"] as StartupFilter[]).map((f) => (
                <SelectItem key={f} value={f}>
                  {t(`services.startupFilters.${f}`)}
                </SelectItem>
              ))}
            </SelectContent>
          </Select>
          <div className="ml-auto flex items-center gap-2">
            {updatedAt && (
              <span className="text-xs text-muted-foreground">
                {t("common.updatedAgo", { time: formatRelativeTime(t, updatedAt) })}
              </span>
            )}
            <Button variant="ghost" size="icon-sm" onClick={refresh} disabled={status === "loading"}>
              <RefreshCw className={status === "loading" ? "animate-spin" : ""} />
            </Button>
          </div>
        </div>

        {status === "loading" && services.length === 0 && (
          <p className="flex items-center gap-2 text-sm text-muted-foreground">
            <Loader2 className="size-4 animate-spin" />
            {t("services.reading")}
          </p>
        )}
        {status === "error" && services.length === 0 && (
          <p className="text-sm text-destructive">{t("services.couldntRead", { error })}</p>
        )}
        {status === "ready" && filtered.length === 0 && (
          <p className="text-sm text-muted-foreground">{t("services.noMatches")}</p>
        )}
        {filtered.length > 0 && (
          <>
            <p className="text-xs text-muted-foreground">
              {t("services.count", { shown: Math.min(limit, filtered.length), total: filtered.length })}
            </p>
            <div className="overflow-x-auto rounded-lg border">
              <div className="min-w-[44rem]">
                <div
                  className={`grid ${GRID} gap-2 border-b bg-muted/50 px-3 py-2 text-xs font-medium text-muted-foreground`}
                >
                  <span>{t("services.columns.service")}</span>
                  <span>{t("services.columns.state")}</span>
                  <span>{t("services.columns.startup")}</span>
                  <span />
                </div>
                {filtered.slice(0, limit).map((s) => (
                  <ServiceRow
                    key={s.name}
                    s={s}
                    t={t}
                    isAdministrator={isAdministrator}
                    busy={busyName === s.name}
                    onAction={handleAction}
                  />
                ))}
              </div>
            </div>
            {filtered.length > limit && (
              <div>
                <Button variant="outline" size="sm" onClick={() => setLimit((l) => l + PAGE_SIZE)}>
                  {t("services.showMore")}
                </Button>
              </div>
            )}
          </>
        )}
      </div>

      <Dialog open={pending !== null} onOpenChange={(open) => !open && setPending(null)}>
        <DialogContent>
          <DialogHeader>
            <DialogTitle>
              {pending && t(`services.confirm.${pending.action}Title`, { name: pending.service.displayName })}
            </DialogTitle>
            <DialogDescription>
              {pending && t(`services.confirm.${pending.action}Description`, { name: pending.service.displayName })}
            </DialogDescription>
          </DialogHeader>
          <DialogFooter>
            <Button variant="outline" onClick={() => setPending(null)}>
              {t("services.cancel")}
            </Button>
            <Button
              variant="destructive"
              onClick={() => {
                const p = pending;
                setPending(null);
                if (p) void perform(p.service, p.action);
              }}
            >
              {t("services.confirm.go")}
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </div>
  );
}
