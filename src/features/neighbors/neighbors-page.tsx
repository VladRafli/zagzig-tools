import { useMemo, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { Eraser, Loader2, Lock, RefreshCw, Search, Trash2 } from "lucide-react";
import { useTranslation } from "react-i18next";
import type { TFunction } from "i18next";
import { toast } from "sonner";

import { AdminRequiredTooltip } from "@/components/admin-required-tooltip";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Checkbox } from "@/components/ui/checkbox";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { useCachedInvoke } from "@/lib/use-cached-invoke";
import { formatRelativeTime } from "@/lib/relative-time";
import { useIsAdministrator } from "@/lib/use-is-administrator";

interface Neighbor {
  ipAddress: string;
  linkLayerAddress: string;
  state: string;
  interfaceIndex: number;
  interfaceAlias: string;
  family: string;
}

const GRID = "grid-cols-[1.4fr_1.2fr_6rem_1.2fr_2.5rem]";

function errorMessage(err: unknown) {
  return err instanceof Error ? err.message : String(err);
}

function stateVariant(state: string) {
  if (state === "Reachable") return "default" as const;
  if (state === "Permanent") return "outline" as const;
  return "secondary" as const;
}

function NeighborRow({
  n,
  t,
  isAdministrator,
  onChanged,
}: {
  n: Neighbor;
  t: TFunction;
  isAdministrator: boolean;
  onChanged: () => void;
}) {
  const [busy, setBusy] = useState(false);

  async function remove() {
    setBusy(true);
    try {
      await invoke("remove_neighbor", {
        interfaceIndex: n.interfaceIndex,
        ipAddress: n.ipAddress,
      });
      toast.success(t("neighbors.removeSuccess", { ip: n.ipAddress }));
      onChanged();
    } catch (err) {
      toast.error(t("neighbors.actionError", { error: errorMessage(err) }));
    } finally {
      setBusy(false);
    }
  }

  return (
    <div className={`grid ${GRID} items-center gap-2 border-b px-3 py-2 text-sm last:border-b-0`}>
      <span className="break-all font-mono text-xs">{n.ipAddress}</span>
      <span className="font-mono text-xs text-muted-foreground">
        {n.linkLayerAddress || t("common.none")}
      </span>
      <Badge variant={stateVariant(n.state)} className="w-fit">
        {n.state}
      </Badge>
      <span className="truncate text-xs text-muted-foreground" title={n.interfaceAlias}>
        {n.interfaceAlias}
      </span>
      <div className="flex justify-end">
        <AdminRequiredTooltip locked={!isAdministrator}>
          <Button
            variant="ghost"
            size="icon-sm"
            aria-label={t("neighbors.remove")}
            disabled={!isAdministrator || busy || n.state === "Permanent"}
            onClick={remove}
          >
            {!isAdministrator ? <Lock /> : busy ? <Loader2 className="animate-spin" /> : <Trash2 />}
          </Button>
        </AdminRequiredTooltip>
      </div>
    </div>
  );
}

export function NeighborsPage() {
  const { t } = useTranslation();
  const { data, status, error, updatedAt, refresh } = useCachedInvoke<Neighbor[]>(
    "zagzig:neighbors",
    "get_neighbors",
    15 * 1000,
  );
  const { isAdministrator } = useIsAdministrator();
  const [query, setQuery] = useState("");
  const [showPermanent, setShowPermanent] = useState(false);
  const [clearing, setClearing] = useState(false);

  const neighbors = data ?? [];
  const rows = useMemo(() => {
    const needle = query.trim().toLowerCase();
    return neighbors
      .filter((n) => showPermanent || n.state !== "Permanent")
      .filter(
        (n) =>
          !needle ||
          [n.ipAddress, n.linkLayerAddress, n.interfaceAlias, n.state].some((f) =>
            f.toLowerCase().includes(needle),
          ),
      )
      .sort(
        (a, b) =>
          a.interfaceIndex - b.interfaceIndex ||
          a.family.localeCompare(b.family) ||
          a.ipAddress.localeCompare(b.ipAddress, undefined, { numeric: true }),
      );
  }, [neighbors, query, showPermanent]);

  async function clearAll() {
    setClearing(true);
    try {
      await invoke("clear_neighbors");
      toast.success(t("neighbors.clearSuccess"));
      refresh();
    } catch (err) {
      toast.error(t("neighbors.actionError", { error: errorMessage(err) }));
    } finally {
      setClearing(false);
    }
  }

  return (
    <div className="flex flex-col gap-6">
      <div>
        <h1 className="text-lg font-semibold">{t("neighbors.title")}</h1>
        <p className="text-sm text-muted-foreground">{t("neighbors.subtitle")}</p>
      </div>

      <div className="flex flex-col gap-3">
        <div className="flex flex-wrap items-center gap-3">
          <div className="relative min-w-48 flex-1">
            <Search className="pointer-events-none absolute top-1/2 left-2.5 size-4 -translate-y-1/2 text-muted-foreground" />
            <Input
              className="pl-8"
              placeholder={t("neighbors.searchPlaceholder")}
              value={query}
              onChange={(e) => setQuery(e.currentTarget.value)}
            />
          </div>
          <div className="flex items-center gap-2">
            <Checkbox
              id="neighborsPermanent"
              checked={showPermanent}
              onCheckedChange={(c) => setShowPermanent(c === true)}
            />
            <Label htmlFor="neighborsPermanent">{t("neighbors.showPermanent")}</Label>
          </div>
          <AdminRequiredTooltip locked={!isAdministrator}>
            <Button variant="outline" size="sm" onClick={clearAll} disabled={!isAdministrator || clearing}>
              {!isAdministrator ? <Lock /> : clearing ? <Loader2 className="animate-spin" /> : <Eraser />}
              {t("neighbors.clear")}
            </Button>
          </AdminRequiredTooltip>
          <div className="flex items-center gap-2">
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

        {status === "loading" && neighbors.length === 0 && (
          <p className="flex items-center gap-2 text-sm text-muted-foreground">
            <Loader2 className="size-4 animate-spin" />
            {t("neighbors.reading")}
          </p>
        )}
        {status === "error" && neighbors.length === 0 && (
          <p className="text-sm text-destructive">{t("neighbors.couldntRead", { error })}</p>
        )}
        {status === "ready" && rows.length === 0 && (
          <p className="text-sm text-muted-foreground">{t("neighbors.none")}</p>
        )}
        {rows.length > 0 && (
          <div className="overflow-x-auto rounded-lg border">
            <div className="min-w-[40rem]">
              <div
                className={`grid ${GRID} gap-2 border-b bg-muted/50 px-3 py-2 text-xs font-medium text-muted-foreground`}
              >
                <span>{t("neighbors.columns.ip")}</span>
                <span>{t("neighbors.columns.mac")}</span>
                <span>{t("neighbors.columns.state")}</span>
                <span>{t("neighbors.columns.interface")}</span>
                <span />
              </div>
              {rows.map((n) => (
                <NeighborRow
                  key={`${n.interfaceIndex}|${n.ipAddress}`}
                  n={n}
                  t={t}
                  isAdministrator={isAdministrator}
                  onChanged={refresh}
                />
              ))}
            </div>
          </div>
        )}
      </div>
    </div>
  );
}
