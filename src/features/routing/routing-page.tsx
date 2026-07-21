import { useMemo, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { Loader2, Lock, Plus, RefreshCw, Trash2 } from "lucide-react";
import { Trans, useTranslation } from "react-i18next";
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
import { Switch } from "@/components/ui/switch";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { useNetworkRoutes, type NetRoute } from "@/features/routing/use-network-routes";
import { formatRelativeTime } from "@/lib/relative-time";
import { useIsAdministrator } from "@/lib/use-is-administrator";
import { AdminRequiredTooltip } from "@/components/admin-required-tooltip";

function RemoveRouteDialog({
  route,
  t,
  isAdministrator,
  onRemoved,
}: {
  route: NetRoute;
  t: TFunction;
  isAdministrator: boolean;
  onRemoved: () => void;
}) {
  const [open, setOpen] = useState(false);
  const [removing, setRemoving] = useState(false);
  const [error, setError] = useState<string | null>(null);

  function handleOpenChange(next: boolean) {
    if (removing) return;
    setOpen(next);
    if (!next) setError(null);
  }

  async function confirmRemove() {
    setRemoving(true);
    setError(null);
    try {
      await invoke("remove_route", {
        destinationPrefix: route.destinationPrefix,
        nextHop: route.nextHop,
        interfaceIndex: route.interfaceIndex,
      });
      setOpen(false);
      toast.success(
        t("routing.remove.success", { destination: route.destinationPrefix }),
      );
      onRemoved();
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setRemoving(false);
    }
  }

  return (
    <Dialog open={open} onOpenChange={handleOpenChange}>
      <AdminRequiredTooltip locked={!isAdministrator}>
        <DialogTrigger
          render={
            <Button
              variant="ghost"
              size="icon-sm"
              aria-label={t("routing.remove.button")}
              disabled={!isAdministrator}
            />
          }
        >
          {isAdministrator ? <Trash2 /> : <Lock />}
        </DialogTrigger>
      </AdminRequiredTooltip>
      <DialogContent>
        <DialogHeader>
          <DialogTitle>{t("routing.remove.title")}</DialogTitle>
          <DialogDescription>
            {t("routing.remove.description", {
              destination: route.destinationPrefix,
              nextHop: route.nextHop,
            })}
          </DialogDescription>
        </DialogHeader>
        {error && <p className="text-sm text-destructive">{error}</p>}
        <DialogFooter>
          <Button
            variant="outline"
            onClick={() => handleOpenChange(false)}
            disabled={removing}
          >
            {t("routing.remove.cancel")}
          </Button>
          <Button
            variant="destructive"
            onClick={confirmRemove}
            disabled={removing}
          >
            {removing && <Loader2 className="animate-spin" />}
            {removing
              ? t("routing.remove.removing")
              : t("routing.remove.confirm")}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}

function RouteTable({
  routes,
  t,
  isAdministrator,
  onRemoved,
}: {
  routes: NetRoute[];
  t: TFunction;
  isAdministrator: boolean;
  onRemoved: () => void;
}) {
  return (
    <div className="overflow-x-auto rounded-lg border">
      <div className="min-w-[42rem]">
        <div className="grid grid-cols-[1.2fr_1fr_1.4fr_4rem_6rem_2.5rem] gap-2 border-b bg-muted/50 px-3 py-2 text-xs font-medium text-muted-foreground">
          <span>{t("routing.columns.destination")}</span>
          <span>{t("routing.columns.nextHop")}</span>
          <span>{t("routing.columns.interface")}</span>
          <span className="text-right">{t("routing.columns.metric")}</span>
          <span>{t("routing.columns.protocol")}</span>
          <span />
        </div>
        {routes.map((route) => (
          <div
            key={`${route.destinationPrefix}|${route.nextHop}|${route.interfaceIndex}`}
            className="grid grid-cols-[1.2fr_1fr_1.4fr_4rem_6rem_2.5rem] items-center gap-2 border-b px-3 py-2 text-sm last:border-b-0"
          >
            <span className="break-all">{route.destinationPrefix}</span>
            <span className="break-all text-muted-foreground">
              {route.nextHop}
            </span>
            <span className="break-all text-muted-foreground">
              {route.interfaceAlias}
            </span>
            <span className="text-right text-muted-foreground">
              {route.routeMetric}
            </span>
            <div className="flex flex-wrap items-center gap-1">
              <Badge variant="secondary">{route.protocol}</Badge>
              {route.store === "PersistentStore" && (
                <Badge variant="outline">
                  {t("routing.persistentBadge")}
                </Badge>
              )}
            </div>
            <div className="flex justify-end">
              <RemoveRouteDialog
                route={route}
                t={t}
                isAdministrator={isAdministrator}
                onRemoved={onRemoved}
              />
            </div>
          </div>
        ))}
      </div>
    </div>
  );
}

interface AddRouteFormState {
  destination: string;
  nextHop: string;
  interfaceIndex: string;
  metric: string;
  persistent: boolean;
}

function emptyAddRouteForm(): AddRouteFormState {
  return {
    destination: "",
    nextHop: "",
    interfaceIndex: "",
    metric: "",
    persistent: true,
  };
}

function AddRouteForm({
  interfaces,
  t,
  isAdministrator,
  onAdded,
}: {
  interfaces: { index: number; alias: string }[];
  t: TFunction;
  isAdministrator: boolean;
  onAdded: () => void;
}) {
  const [form, setForm] = useState<AddRouteFormState>(emptyAddRouteForm);
  const [adding, setAdding] = useState(false);
  const [error, setError] = useState<string | null>(null);

  function update<K extends keyof AddRouteFormState>(
    key: K,
    value: AddRouteFormState[K],
  ) {
    setForm((prev) => ({ ...prev, [key]: value }));
  }

  const canAdd =
    isAdministrator &&
    form.destination.trim().length > 0 &&
    form.nextHop.trim().length > 0 &&
    form.interfaceIndex.length > 0 &&
    !adding;

  async function addRoute() {
    if (!canAdd) return;
    setAdding(true);
    setError(null);
    try {
      await invoke("add_route", {
        destinationPrefix: form.destination.trim(),
        nextHop: form.nextHop.trim(),
        interfaceIndex: Number(form.interfaceIndex),
        routeMetric: form.metric.trim() ? Number(form.metric.trim()) : null,
        persistent: form.persistent,
      });
      toast.success(
        t("routing.addSuccess", { destination: form.destination.trim() }),
      );
      setForm(emptyAddRouteForm());
      onAdded();
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setAdding(false);
    }
  }

  return (
    <Card>
      <CardHeader>
        <div className="flex items-center gap-2">
          <CardTitle>{t("routing.newRoute")}</CardTitle>
          {!isAdministrator && (
            <Badge variant="outline" className="gap-1">
              <Lock className="size-3" />
              {t("common.administratorOnly")}
            </Badge>
          )}
        </div>
        <CardDescription>{t("routing.newRouteDescription")}</CardDescription>
      </CardHeader>
      <CardContent className="flex flex-col gap-4">
        <div className="grid gap-4 sm:grid-cols-2">
          <div className="flex flex-col gap-1.5">
            <Label htmlFor="destination">{t("routing.destinationLabel")}</Label>
            <Input
              id="destination"
              placeholder={t("routing.destinationPlaceholder")}
              value={form.destination}
              onChange={(e) => update("destination", e.currentTarget.value)}
              disabled={!isAdministrator}
            />
          </div>
          <div className="flex flex-col gap-1.5">
            <Label htmlFor="nextHop">{t("routing.nextHopLabel")}</Label>
            <Input
              id="nextHop"
              placeholder={t("routing.nextHopPlaceholder")}
              value={form.nextHop}
              onChange={(e) => update("nextHop", e.currentTarget.value)}
              disabled={!isAdministrator}
            />
          </div>
          <div className="flex flex-col gap-1.5">
            <Label>{t("routing.interfaceLabel")}</Label>
            <Select
              value={form.interfaceIndex}
              onValueChange={(value) => update("interfaceIndex", value as string)}
              disabled={!isAdministrator}
            >
              <SelectTrigger className="w-full">
                <SelectValue placeholder={t("routing.interfacePlaceholder")} />
              </SelectTrigger>
              <SelectContent>
                {interfaces.map((iface) => (
                  <SelectItem key={iface.index} value={String(iface.index)}>
                    {iface.alias}
                  </SelectItem>
                ))}
              </SelectContent>
            </Select>
          </div>
          <div className="flex flex-col gap-1.5">
            <Label htmlFor="metric">{t("routing.metricLabel")}</Label>
            <Input
              id="metric"
              type="number"
              min={0}
              value={form.metric}
              onChange={(e) => update("metric", e.currentTarget.value)}
              disabled={!isAdministrator}
            />
          </div>
        </div>

        <div className="flex items-center justify-between gap-2">
          <Label htmlFor="persistent">{t("routing.persistentLabel")}</Label>
          <Switch
            id="persistent"
            checked={form.persistent}
            onCheckedChange={(checked) => update("persistent", checked)}
            disabled={!isAdministrator}
          />
        </div>

        {error && <p className="text-sm text-destructive">{error}</p>}

        <div>
          <AdminRequiredTooltip locked={!isAdministrator}>
            <Button onClick={addRoute} disabled={!canAdd}>
              {!isAdministrator ? (
                <Lock />
              ) : adding ? (
                <Loader2 className="animate-spin" />
              ) : (
                <Plus />
              )}
              {adding ? t("routing.adding") : t("routing.addRoute")}
            </Button>
          </AdminRequiredTooltip>
        </div>
      </CardContent>
    </Card>
  );
}

export function RoutingPage() {
  const { t } = useTranslation();
  const routes = useNetworkRoutes();
  const { isAdministrator } = useIsAdministrator();
  const [showSystemRoutes, setShowSystemRoutes] = useState(false);

  const visibleRoutes = showSystemRoutes
    ? routes.routes
    : routes.routes.filter((route) => route.protocol !== "Local");

  const interfaces = useMemo(() => {
    const byIndex = new Map<number, string>();
    for (const route of routes.routes) {
      if (!byIndex.has(route.interfaceIndex)) {
        byIndex.set(route.interfaceIndex, route.interfaceAlias);
      }
    }
    return Array.from(byIndex, ([index, alias]) => ({ index, alias })).sort(
      (a, b) => a.alias.localeCompare(b.alias),
    );
  }, [routes.routes]);

  return (
    <div className="flex flex-col gap-6">
      <div>
        <h1 className="text-lg font-semibold">{t("routing.title")}</h1>
        <p className="text-sm text-muted-foreground">
          <Trans
            i18nKey="routing.subtitle"
            components={{ 1: <code className="text-xs" /> }}
          />
        </p>
      </div>

      <div className="flex flex-col gap-3">
        <div className="flex flex-wrap items-center justify-between gap-2">
          <h2 className="text-sm font-medium text-muted-foreground">
            {t("routing.currentRoutes")}
          </h2>
          <div className="flex items-center gap-3">
            <div className="flex items-center gap-2">
              <Switch
                id="showSystemRoutes"
                size="sm"
                checked={showSystemRoutes}
                onCheckedChange={setShowSystemRoutes}
              />
              <Label
                htmlFor="showSystemRoutes"
                className="text-xs text-muted-foreground"
              >
                {t("routing.showSystemRoutes")}
              </Label>
            </div>
            {routes.updatedAt && (
              <span className="text-xs text-muted-foreground">
                {t("common.updatedAgo", {
                  time: formatRelativeTime(t, routes.updatedAt),
                })}
              </span>
            )}
            <Button
              variant="ghost"
              size="icon-sm"
              onClick={routes.refresh}
              disabled={routes.status === "loading"}
            >
              <RefreshCw
                className={routes.status === "loading" ? "animate-spin" : ""}
              />
            </Button>
          </div>
        </div>

        {routes.status === "loading" && routes.routes.length === 0 && (
          <p className="flex items-center gap-2 text-sm text-muted-foreground">
            <Loader2 className="size-4 animate-spin" />
            {t("routing.reading")}
          </p>
        )}
        {routes.status === "error" && routes.routes.length === 0 && (
          <p className="text-sm text-destructive">
            {t("routing.couldntRead", { error: routes.error })}
          </p>
        )}
        {routes.status === "ready" && visibleRoutes.length === 0 && (
          <p className="text-sm text-muted-foreground">
            {t("routing.noneConfigured")}
          </p>
        )}
        {visibleRoutes.length > 0 && (
          <RouteTable
            routes={visibleRoutes}
            t={t}
            isAdministrator={isAdministrator}
            onRemoved={routes.refresh}
          />
        )}
      </div>

      <AddRouteForm
        interfaces={interfaces}
        t={t}
        isAdministrator={isAdministrator}
        onAdded={routes.refresh}
      />
    </div>
  );
}
