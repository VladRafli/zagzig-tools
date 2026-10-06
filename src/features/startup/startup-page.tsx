import { useMemo, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { FolderOpen, Loader2, RefreshCw, Search } from "lucide-react";
import { useTranslation } from "react-i18next";
import type { TFunction } from "i18next";
import { toast } from "sonner";

import { AdminRequiredTooltip } from "@/components/admin-required-tooltip";
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
import { Switch } from "@/components/ui/switch";
import { formatRelativeTime } from "@/lib/relative-time";
import { useCachedInvoke } from "@/lib/use-cached-invoke";
import { useIsAdministrator } from "@/lib/use-is-administrator";

interface StartupItem {
  source: string;
  sourceLabel: string;
  machineWide: boolean;
  name: string;
  command: string;
  path: string | null;
  fileExists: boolean;
  company: string | null;
  description: string | null;
  enabled: boolean;
}

type Filter = "all" | "enabled" | "disabled";

const GRID = "grid-cols-[2.5rem_1.2fr_1.6fr_9rem_2.5rem]";

function errorMessage(err: unknown) {
  return err instanceof Error ? err.message : String(err);
}

// Windows can't be asked about files under the Store-app folder without
// special rights, so "not found" there would be a false alarm.
function trulyMissing(item: StartupItem) {
  return Boolean(item.path) && !item.fileExists && !/\\WindowsApps\\/i.test(item.path ?? "");
}

function StartupRow({
  item,
  t,
  isAdministrator,
  onChanged,
}: {
  item: StartupItem;
  t: TFunction;
  isAdministrator: boolean;
  onChanged: () => void;
}) {
  const [busy, setBusy] = useState(false);
  const locked = item.machineWide && !isAdministrator;

  async function toggle(next: boolean) {
    setBusy(true);
    try {
      await invoke("set_startup_item_enabled", { source: item.source, name: item.name, enabled: next });
      toast.success(t(next ? "startup.enabledSuccess" : "startup.disabledSuccess", { name: item.name }));
      onChanged();
    } catch (err) {
      toast.error(t("startup.toggleError", { error: errorMessage(err) }));
    } finally {
      setBusy(false);
    }
  }

  return (
    <div
      className={`grid ${GRID} items-center gap-2 border-b px-3 py-2 text-sm last:border-b-0 ${
        item.enabled ? "" : "text-muted-foreground"
      }`}
    >
      <AdminRequiredTooltip locked={locked}>
        <Switch checked={item.enabled} disabled={locked || busy} onCheckedChange={toggle} />
      </AdminRequiredTooltip>
      <div className="min-w-0">
        <div className="flex flex-wrap items-center gap-2">
          <span className="font-medium break-all">{item.name}</span>
          {trulyMissing(item) && <Badge variant="destructive">{t("startup.missing")}</Badge>}
        </div>
        <p className="truncate text-xs text-muted-foreground" title={item.description ?? undefined}>
          {[item.description, item.company].filter(Boolean).join(" — ") || t("common.none")}
        </p>
      </div>
      <span className="truncate font-mono text-xs text-muted-foreground" title={item.command}>
        {item.command}
      </span>
      <Badge variant={item.machineWide ? "secondary" : "outline"} className="w-fit">
        {item.sourceLabel}
      </Badge>
      <div className="flex justify-end">
        <Button
          variant="ghost"
          size="icon-sm"
          aria-label={t("startup.reveal")}
          title={t("startup.reveal")}
          disabled={!item.path || !item.fileExists}
          onClick={() => item.path && void invoke("reveal_in_explorer", { path: item.path }).catch(() => {})}
        >
          <FolderOpen />
        </Button>
      </div>
    </div>
  );
}

export function StartupPage() {
  const { t } = useTranslation();
  const { data, status, error, updatedAt, refresh } = useCachedInvoke<StartupItem[]>(
    "zagzig:startup",
    "get_startup_items",
    10 * 1000,
  );
  const { isAdministrator } = useIsAdministrator();
  const [query, setQuery] = useState("");
  const [filter, setFilter] = useState<Filter>("all");

  const items = data ?? [];
  const rows = useMemo(() => {
    const needle = query.trim().toLowerCase();
    return items.filter(
      (i) =>
        (filter === "all" || (filter === "enabled") === i.enabled) &&
        (!needle ||
          [i.name, i.command, i.company, i.description, i.sourceLabel].some((f) =>
            f?.toLowerCase().includes(needle),
          )),
    );
  }, [items, query, filter]);

  return (
    <div className="flex flex-col gap-6">
      <div>
        <h1 className="text-lg font-semibold">{t("startup.title")}</h1>
        <p className="text-sm text-muted-foreground">{t("startup.subtitle")}</p>
      </div>

      <div className="flex flex-col gap-3">
        <div className="flex flex-wrap items-center gap-2">
          <div className="relative min-w-48 flex-1">
            <Search className="pointer-events-none absolute top-1/2 left-2.5 size-4 -translate-y-1/2 text-muted-foreground" />
            <Input
              className="pl-8"
              placeholder={t("startup.searchPlaceholder")}
              value={query}
              onChange={(e) => setQuery(e.currentTarget.value)}
            />
          </div>
          <Select value={filter} onValueChange={(v) => setFilter(v as Filter)}>
            <SelectTrigger size="sm" className="w-36" aria-label={t("startup.filter")}>
              <SelectValue>{(v: Filter) => t(`startup.filters.${v}`)}</SelectValue>
            </SelectTrigger>
            <SelectContent>
              {(["all", "enabled", "disabled"] as Filter[]).map((f) => (
                <SelectItem key={f} value={f}>
                  {t(`startup.filters.${f}`)}
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

        {status === "loading" && items.length === 0 && (
          <p className="flex items-center gap-2 text-sm text-muted-foreground">
            <Loader2 className="size-4 animate-spin" />
            {t("startup.reading")}
          </p>
        )}
        {status === "error" && items.length === 0 && (
          <p className="text-sm text-destructive">{t("startup.couldntRead", { error })}</p>
        )}
        {status === "ready" && rows.length === 0 && (
          <p className="text-sm text-muted-foreground">{t("startup.none")}</p>
        )}
        {rows.length > 0 && (
          <div className="overflow-x-auto rounded-lg border">
            <div className="min-w-[52rem]">
              <div
                className={`grid ${GRID} gap-2 border-b bg-muted/50 px-3 py-2 text-xs font-medium text-muted-foreground`}
              >
                <span>{t("startup.columns.on")}</span>
                <span>{t("startup.columns.program")}</span>
                <span>{t("startup.columns.command")}</span>
                <span>{t("startup.columns.location")}</span>
                <span />
              </div>
              {rows.map((item) => (
                <StartupRow
                  key={`${item.source}|${item.name}`}
                  item={item}
                  t={t}
                  isAdministrator={isAdministrator}
                  onChanged={refresh}
                />
              ))}
            </div>
          </div>
        )}
        <p className="text-xs text-muted-foreground">{t("startup.scopeNote")}</p>
      </div>
    </div>
  );
}
