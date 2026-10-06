import { useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { Loader2, Lock, Plus, RefreshCw, Trash2 } from "lucide-react";
import { useTranslation } from "react-i18next";
import type { TFunction } from "i18next";
import { toast } from "sonner";

import { AdminRequiredTooltip } from "@/components/admin-required-tooltip";
import { Badge } from "@/components/ui/badge";
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
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import {
  PORT_PROXY_KINDS,
  usePortProxy,
  type PortProxyKind,
  type PortProxyRule,
} from "@/features/port-proxy/use-port-proxy";
import { formatRelativeTime } from "@/lib/relative-time";
import { useIsAdministrator } from "@/lib/use-is-administrator";

function errorMessage(err: unknown) {
  return err instanceof Error ? err.message : String(err);
}

function formatEndpoint(address: string, port: number) {
  return address.includes(":") ? `[${address}]:${port}` : `${address}:${port}`;
}

function RemoveRuleDialog({
  rule,
  t,
  isAdministrator,
  onRemoved,
}: {
  rule: PortProxyRule;
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
      await invoke("remove_portproxy_rule", {
        kind: rule.kind,
        listenAddress: rule.listenAddress,
        listenPort: rule.listenPort,
      });
      setOpen(false);
      toast.success(t("portProxy.remove.success", { port: rule.listenPort }));
      onRemoved();
    } catch (err) {
      setError(errorMessage(err));
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
              aria-label={t("portProxy.remove.button")}
              disabled={!isAdministrator}
            />
          }
        >
          {isAdministrator ? <Trash2 /> : <Lock />}
        </DialogTrigger>
      </AdminRequiredTooltip>
      <DialogContent>
        <DialogHeader>
          <DialogTitle>{t("portProxy.remove.title")}</DialogTitle>
          <DialogDescription>
            {t("portProxy.remove.description", {
              listen: formatEndpoint(rule.listenAddress, rule.listenPort),
              connect: formatEndpoint(rule.connectAddress, rule.connectPort),
            })}
          </DialogDescription>
        </DialogHeader>
        {error && <p className="text-sm text-destructive">{error}</p>}
        <DialogFooter>
          <Button variant="outline" onClick={() => handleOpenChange(false)} disabled={removing}>
            {t("portProxy.cancel")}
          </Button>
          <Button variant="destructive" onClick={confirmRemove} disabled={removing}>
            {removing && <Loader2 className="animate-spin" />}
            {removing ? t("portProxy.remove.removing") : t("portProxy.remove.confirm")}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}

const GRID = "grid-cols-[6.5rem_1fr_1fr_2.5rem]";

function RuleTable({
  rules,
  t,
  isAdministrator,
  onChanged,
}: {
  rules: PortProxyRule[];
  t: TFunction;
  isAdministrator: boolean;
  onChanged: () => void;
}) {
  return (
    <div className="overflow-x-auto rounded-lg border">
      <div className="min-w-[36rem]">
        <div
          className={`grid ${GRID} gap-2 border-b bg-muted/50 px-3 py-2 text-xs font-medium text-muted-foreground`}
        >
          <span>{t("portProxy.columns.type")}</span>
          <span>{t("portProxy.columns.listen")}</span>
          <span>{t("portProxy.columns.connect")}</span>
          <span />
        </div>
        {rules.map((rule) => (
          <div
            key={`${rule.kind}|${rule.listenAddress}|${rule.listenPort}`}
            className={`grid ${GRID} items-center gap-2 border-b px-3 py-2 text-sm last:border-b-0`}
          >
            <Badge variant="secondary" className="w-fit">
              {t(`portProxy.kinds.${rule.kind}`)}
            </Badge>
            <span className="break-all font-mono text-xs">
              {formatEndpoint(rule.listenAddress, rule.listenPort)}
            </span>
            <span className="break-all font-mono text-xs text-muted-foreground">
              {formatEndpoint(rule.connectAddress, rule.connectPort)}
            </span>
            <div className="flex justify-end">
              <RemoveRuleDialog
                rule={rule}
                t={t}
                isAdministrator={isAdministrator}
                onRemoved={onChanged}
              />
            </div>
          </div>
        ))}
      </div>
    </div>
  );
}

interface FormState {
  kind: PortProxyKind;
  listenAddress: string;
  listenPort: string;
  connectAddress: string;
  connectPort: string;
}

function emptyForm(): FormState {
  return {
    kind: "v4tov4",
    listenAddress: "0.0.0.0",
    listenPort: "",
    connectAddress: "",
    connectPort: "",
  };
}

function toPort(value: string): number | null {
  const n = Number(value.trim());
  return Number.isInteger(n) && n >= 1 && n <= 65535 ? n : null;
}

function AddRuleForm({
  t,
  isAdministrator,
  onAdded,
}: {
  t: TFunction;
  isAdministrator: boolean;
  onAdded: () => void;
}) {
  const [form, setForm] = useState<FormState>(emptyForm);
  const [adding, setAdding] = useState(false);
  const [error, setError] = useState<string | null>(null);

  function update<K extends keyof FormState>(key: K, value: FormState[K]) {
    setForm((prev) => ({ ...prev, [key]: value }));
  }

  const listenPort = toPort(form.listenPort);
  const connectPort = toPort(form.connectPort);
  const canAdd =
    isAdministrator &&
    form.listenAddress.trim().length > 0 &&
    form.connectAddress.trim().length > 0 &&
    listenPort !== null &&
    connectPort !== null &&
    !adding;

  async function addRule() {
    if (!canAdd) return;
    setAdding(true);
    setError(null);
    try {
      await invoke("add_portproxy_rule", {
        kind: form.kind,
        listenAddress: form.listenAddress.trim(),
        listenPort,
        connectAddress: form.connectAddress.trim(),
        connectPort,
      });
      toast.success(t("portProxy.addSuccess", { port: listenPort }));
      setForm(emptyForm());
      onAdded();
    } catch (err) {
      setError(errorMessage(err));
    } finally {
      setAdding(false);
    }
  }

  return (
    <Card>
      <CardHeader>
        <div className="flex items-center gap-2">
          <CardTitle>{t("portProxy.newRule")}</CardTitle>
          {!isAdministrator && (
            <Badge variant="outline" className="gap-1">
              <Lock className="size-3" />
              {t("common.administratorOnly")}
            </Badge>
          )}
        </div>
        <CardDescription>{t("portProxy.newRuleDescription")}</CardDescription>
      </CardHeader>
      <CardContent className="flex flex-col gap-4">
        <div className="flex flex-col gap-1.5 sm:max-w-xs">
          <Label htmlFor="portProxyKind">{t("portProxy.typeLabel")}</Label>
          <Select
            value={form.kind}
            onValueChange={(v) => update("kind", v as PortProxyKind)}
            disabled={!isAdministrator}
          >
            <SelectTrigger id="portProxyKind" size="sm" className="w-full">
              <SelectValue>
                {(v: PortProxyKind) => t(`portProxy.kinds.${v}`)}
              </SelectValue>
            </SelectTrigger>
            <SelectContent>
              {PORT_PROXY_KINDS.map((k) => (
                <SelectItem key={k} value={k}>
                  {t(`portProxy.kinds.${k}`)}
                </SelectItem>
              ))}
            </SelectContent>
          </Select>
        </div>
        <div className="grid gap-4 sm:grid-cols-2">
          <div className="flex flex-col gap-1.5">
            <Label htmlFor="ppListenAddress">{t("portProxy.listenAddressLabel")}</Label>
            <Input
              id="ppListenAddress"
              placeholder="0.0.0.0"
              value={form.listenAddress}
              onChange={(e) => update("listenAddress", e.currentTarget.value)}
              disabled={!isAdministrator}
            />
          </div>
          <div className="flex flex-col gap-1.5">
            <Label htmlFor="ppListenPort">{t("portProxy.listenPortLabel")}</Label>
            <Input
              id="ppListenPort"
              placeholder="8080"
              inputMode="numeric"
              value={form.listenPort}
              onChange={(e) => update("listenPort", e.currentTarget.value)}
              disabled={!isAdministrator}
            />
          </div>
          <div className="flex flex-col gap-1.5">
            <Label htmlFor="ppConnectAddress">{t("portProxy.connectAddressLabel")}</Label>
            <Input
              id="ppConnectAddress"
              placeholder={t("portProxy.connectAddressPlaceholder")}
              value={form.connectAddress}
              onChange={(e) => update("connectAddress", e.currentTarget.value)}
              disabled={!isAdministrator}
            />
          </div>
          <div className="flex flex-col gap-1.5">
            <Label htmlFor="ppConnectPort">{t("portProxy.connectPortLabel")}</Label>
            <Input
              id="ppConnectPort"
              placeholder="80"
              inputMode="numeric"
              value={form.connectPort}
              onChange={(e) => update("connectPort", e.currentTarget.value)}
              disabled={!isAdministrator}
            />
          </div>
        </div>
        <p className="text-xs text-muted-foreground">{t("portProxy.firewallNote")}</p>
        {error && <p className="text-sm text-destructive">{error}</p>}
        <div>
          <AdminRequiredTooltip locked={!isAdministrator}>
            <Button onClick={addRule} disabled={!canAdd}>
              {!isAdministrator ? (
                <Lock />
              ) : adding ? (
                <Loader2 className="animate-spin" />
              ) : (
                <Plus />
              )}
              {adding ? t("portProxy.adding") : t("portProxy.addRule")}
            </Button>
          </AdminRequiredTooltip>
        </div>
      </CardContent>
    </Card>
  );
}

export function PortProxyPage() {
  const { t } = useTranslation();
  const { rules, status, error, updatedAt, refresh } = usePortProxy();
  const { isAdministrator } = useIsAdministrator();

  return (
    <div className="flex flex-col gap-6">
      <div>
        <h1 className="text-lg font-semibold">{t("portProxy.title")}</h1>
        <p className="text-sm text-muted-foreground">{t("portProxy.subtitle")}</p>
      </div>

      <div className="flex flex-col gap-3">
        <div className="flex items-center justify-between">
          <div className="flex items-center gap-2">
            {updatedAt && (
              <span className="text-xs text-muted-foreground">
                {t("common.updatedAgo", {
                  time: formatRelativeTime(t, updatedAt),
                })}
              </span>
            )}
          </div>
          <Button
            variant="ghost"
            size="icon-sm"
            onClick={refresh}
            disabled={status === "loading"}
          >
            <RefreshCw className={status === "loading" ? "animate-spin" : ""} />
          </Button>
        </div>

        {status === "loading" && rules.length === 0 && (
          <p className="flex items-center gap-2 text-sm text-muted-foreground">
            <Loader2 className="size-4 animate-spin" />
            {t("portProxy.reading")}
          </p>
        )}
        {status === "error" && rules.length === 0 && (
          <p className="text-sm text-destructive">
            {t("portProxy.couldntRead", { error })}
          </p>
        )}
        {status === "ready" && rules.length === 0 && (
          <p className="text-sm text-muted-foreground">{t("portProxy.noneConfigured")}</p>
        )}
        {rules.length > 0 && (
          <RuleTable
            rules={rules}
            t={t}
            isAdministrator={isAdministrator}
            onChanged={refresh}
          />
        )}
      </div>

      <AddRuleForm t={t} isAdministrator={isAdministrator} onAdded={refresh} />
    </div>
  );
}
