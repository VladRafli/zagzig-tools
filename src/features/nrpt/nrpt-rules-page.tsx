import { useState } from "react";
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
import { Textarea } from "@/components/ui/textarea";
import { Badge } from "@/components/ui/badge";
import { Separator } from "@/components/ui/separator";
import { Switch } from "@/components/ui/switch";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import {
  useSystemNrptRules,
  type SystemNrptRule,
} from "@/features/nrpt/use-system-nrpt-rules";
import { DetailList, DetailRow } from "@/components/detail-list";
import { CollapsibleDetails } from "@/components/collapsible-details";
import { AdminRequiredTooltip } from "@/components/admin-required-tooltip";
import { formatRelativeTime } from "@/lib/relative-time";
import { useIsAdministrator } from "@/lib/use-is-administrator";

interface NrptRule {
  id: string;
  namespace: string;
  servers: string[];
  comment: string;
  nameEncoding: string;
  dnsSecEnabled: boolean;
  dnsSecValidationRequired: boolean;
  dnsSecQueryIpsecEncryption: string;
  dnsSecQueryIpsecRequired: boolean;
  directAccessEnabled: boolean;
  directAccessDnsServers: string[];
  directAccessProxyName: string;
  directAccessProxyType: string;
  directAccessQueryIpsecEncryption: string;
  directAccessQueryIpsecRequired: boolean;
  ipsecCaRestriction: string;
}

const NAME_ENCODING_OPTIONS = [
  "Disable",
  "Utf8WithMapping",
  "Utf8WithoutMapping",
  "Punycode",
] as const;

const DA_PROXY_TYPE_OPTIONS = ["NoProxy", "UseDefault", "UseProxyName"] as const;

function emptyNewRuleForm() {
  return {
    namespace: "",
    servers: "",
    comment: "",
    nameEncoding: "Disable" as string,
    dnsSecEnabled: false,
    dnsSecValidationRequired: false,
    dnsSecQueryIpsecEncryption: "",
    dnsSecQueryIpsecRequired: false,
    directAccessEnabled: false,
    directAccessDnsServers: "",
    directAccessProxyName: "",
    directAccessProxyType: "NoProxy" as string,
    directAccessQueryIpsecEncryption: "",
    directAccessQueryIpsecRequired: false,
    ipsecCaRestriction: "",
  };
}

type NewRuleFormState = ReturnType<typeof emptyNewRuleForm>;

function RemoveRuleDialog({
  rule,
  t,
  isAdministrator,
  onRemoved,
}: {
  rule: SystemNrptRule;
  t: TFunction;
  isAdministrator: boolean;
  onRemoved: () => void;
}) {
  const [open, setOpen] = useState(false);
  const [removing, setRemoving] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const namespaceLabel = rule.namespace.join(", ") || rule.name;

  function handleOpenChange(next: boolean) {
    if (removing) return;
    setOpen(next);
    if (!next) setError(null);
  }

  async function confirmRemove() {
    setRemoving(true);
    setError(null);
    try {
      await invoke("remove_nrpt_rule", { name: rule.name });
      setOpen(false);
      toast.success(t("nrpt.remove.success", { namespace: namespaceLabel }));
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
              aria-label={t("nrpt.remove.button")}
              disabled={!isAdministrator}
            />
          }
        >
          {isAdministrator ? <Trash2 /> : <Lock />}
        </DialogTrigger>
      </AdminRequiredTooltip>
      <DialogContent>
        <DialogHeader>
          <DialogTitle>{t("nrpt.remove.title")}</DialogTitle>
          <DialogDescription>
            {t("nrpt.remove.description", { namespace: namespaceLabel })}
          </DialogDescription>
        </DialogHeader>
        {error && <p className="text-sm text-destructive">{error}</p>}
        <DialogFooter>
          <Button
            variant="outline"
            onClick={() => handleOpenChange(false)}
            disabled={removing}
          >
            {t("nrpt.remove.cancel")}
          </Button>
          <Button
            variant="destructive"
            onClick={confirmRemove}
            disabled={removing}
          >
            {removing && <Loader2 className="animate-spin" />}
            {removing ? t("nrpt.remove.removing") : t("nrpt.remove.confirm")}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}

function SystemRuleCard({
  rule,
  t,
  isAdministrator,
  onRemoved,
}: {
  rule: SystemNrptRule;
  t: TFunction;
  isAdministrator: boolean;
  onRemoved: () => void;
}) {
  return (
    <Card>
      <CardContent className="flex flex-col gap-3">
        <div className="flex items-start justify-between gap-2">
          <div className="flex flex-wrap items-center gap-1.5">
            {rule.namespace.map((ns) => (
              <Badge key={ns}>{ns}</Badge>
            ))}
            {rule.nameServers.map((server) => (
              <Badge key={server} variant="secondary">
                {server}
              </Badge>
            ))}
            {rule.displayName && (
              <span className="text-sm text-muted-foreground">
                {rule.displayName}
              </span>
            )}
          </div>
          <RemoveRuleDialog
            rule={rule}
            t={t}
            isAdministrator={isAdministrator}
            onRemoved={onRemoved}
          />
        </div>

        <CollapsibleDetails>
          <Separator className="mb-3" />
          <DetailList>
            <DetailRow label={t("nrpt.fields.ruleId")} value={rule.name} />
            <DetailRow
              label={t("nrpt.fields.comment")}
              value={rule.comment}
            />
            <DetailRow
              label={t("nrpt.fields.nameServers")}
              value={rule.nameServers}
            />
            <DetailRow
              label={t("nrpt.fields.nameEncoding")}
              value={rule.nameEncoding}
            />
            <DetailRow label={t("nrpt.fields.version")} value={rule.version} />
            <DetailRow
              label={t("nrpt.fields.dnsSecEnabled")}
              value={rule.dnsSecEnabled}
            />
            <DetailRow
              label={t("nrpt.fields.dnsSecValidationRequired")}
              value={rule.dnsSecValidationRequired}
            />
            <DetailRow
              label={t("nrpt.fields.dnsSecQueryIpsecEncryption")}
              value={rule.dnsSecQueryIpsecEncryption}
            />
            <DetailRow
              label={t("nrpt.fields.dnsSecQueryIpsecRequired")}
              value={rule.dnsSecQueryIpsecRequired}
            />
            <DetailRow
              label={t("nrpt.fields.directAccessEnabled")}
              value={rule.directAccessEnabled}
            />
            <DetailRow
              label={t("nrpt.fields.directAccessDnsServers")}
              value={rule.directAccessDnsServers}
            />
            <DetailRow
              label={t("nrpt.fields.directAccessProxyName")}
              value={rule.directAccessProxyName}
            />
            <DetailRow
              label={t("nrpt.fields.directAccessProxyType")}
              value={rule.directAccessProxyType}
            />
            <DetailRow
              label={t("nrpt.fields.directAccessQueryIpsecEncryption")}
              value={rule.directAccessQueryIpsecEncryption}
            />
            <DetailRow
              label={t("nrpt.fields.directAccessQueryIpsecRequired")}
              value={rule.directAccessQueryIpsecRequired}
            />
            <DetailRow
              label={t("nrpt.fields.ipsecCaRestriction")}
              value={rule.ipsecCaRestriction}
            />
          </DetailList>
        </CollapsibleDetails>
      </CardContent>
    </Card>
  );
}

function parseServers(raw: string): string[] {
  return raw
    .split(/[\s,]+/)
    .map((s) => s.trim())
    .filter(Boolean);
}

function NewRuleForm({
  t,
  onAdd,
}: {
  t: TFunction;
  onAdd: (rule: NrptRule) => void;
}) {
  const [form, setForm] = useState<NewRuleFormState>(emptyNewRuleForm);

  function update<K extends keyof NewRuleFormState>(
    key: K,
    value: NewRuleFormState[K],
  ) {
    setForm((prev) => ({ ...prev, [key]: value }));
  }

  const servers = parseServers(form.servers);
  const canAdd = form.namespace.trim().length > 0 && servers.length > 0;

  function addRule() {
    if (!canAdd) return;
    onAdd({
      id: crypto.randomUUID(),
      namespace: form.namespace.trim(),
      servers,
      comment: form.comment.trim(),
      nameEncoding: form.nameEncoding,
      dnsSecEnabled: form.dnsSecEnabled,
      dnsSecValidationRequired: form.dnsSecValidationRequired,
      dnsSecQueryIpsecEncryption: form.dnsSecQueryIpsecEncryption.trim(),
      dnsSecQueryIpsecRequired: form.dnsSecQueryIpsecRequired,
      directAccessEnabled: form.directAccessEnabled,
      directAccessDnsServers: parseServers(form.directAccessDnsServers),
      directAccessProxyName: form.directAccessProxyName.trim(),
      directAccessProxyType: form.directAccessProxyType,
      directAccessQueryIpsecEncryption:
        form.directAccessQueryIpsecEncryption.trim(),
      directAccessQueryIpsecRequired: form.directAccessQueryIpsecRequired,
      ipsecCaRestriction: form.ipsecCaRestriction.trim(),
    });
    setForm(emptyNewRuleForm());
  }

  return (
    <Card>
      <CardHeader>
        <CardTitle>{t("nrpt.newRule")}</CardTitle>
        <CardDescription>{t("nrpt.newRuleDescription")}</CardDescription>
      </CardHeader>
      <CardContent className="flex flex-col gap-4">
        <div className="flex flex-col gap-1.5">
          <Label htmlFor="namespace">{t("nrpt.namespaceLabel")}</Label>
          <Input
            id="namespace"
            placeholder={t("nrpt.namespacePlaceholder")}
            value={form.namespace}
            onChange={(e) => update("namespace", e.currentTarget.value)}
          />
        </div>
        <div className="flex flex-col gap-1.5">
          <Label htmlFor="servers">{t("nrpt.dnsServersLabel")}</Label>
          <Textarea
            id="servers"
            placeholder={t("nrpt.dnsServersPlaceholder")}
            value={form.servers}
            onChange={(e) => update("servers", e.currentTarget.value)}
          />
        </div>
        <div className="flex flex-col gap-1.5">
          <Label htmlFor="comment">{t("nrpt.fields.comment")}</Label>
          <Input
            id="comment"
            value={form.comment}
            onChange={(e) => update("comment", e.currentTarget.value)}
          />
        </div>

        <CollapsibleDetails>
          <Separator className="mb-4" />
          <div className="grid gap-4 sm:grid-cols-2">
            <div className="flex flex-col gap-1.5">
              <Label>{t("nrpt.fields.nameEncoding")}</Label>
              <Select
                value={form.nameEncoding}
                onValueChange={(value) =>
                  update("nameEncoding", value as string)
                }
              >
                <SelectTrigger className="w-full">
                  <SelectValue />
                </SelectTrigger>
                <SelectContent>
                  {NAME_ENCODING_OPTIONS.map((option) => (
                    <SelectItem key={option} value={option}>
                      {option}
                    </SelectItem>
                  ))}
                </SelectContent>
              </Select>
            </div>

            <div className="flex flex-col gap-1.5">
              <Label htmlFor="ipsecCaRestriction">
                {t("nrpt.fields.ipsecCaRestriction")}
              </Label>
              <Input
                id="ipsecCaRestriction"
                value={form.ipsecCaRestriction}
                onChange={(e) =>
                  update("ipsecCaRestriction", e.currentTarget.value)
                }
              />
            </div>

            <div className="flex items-center justify-between gap-2 sm:col-span-2">
              <Label htmlFor="dnsSecEnabled">
                {t("nrpt.fields.dnsSecEnabled")}
              </Label>
              <Switch
                id="dnsSecEnabled"
                checked={form.dnsSecEnabled}
                onCheckedChange={(checked) =>
                  update("dnsSecEnabled", checked)
                }
              />
            </div>
            <div className="flex items-center justify-between gap-2">
              <Label htmlFor="dnsSecValidationRequired">
                {t("nrpt.fields.dnsSecValidationRequired")}
              </Label>
              <Switch
                id="dnsSecValidationRequired"
                checked={form.dnsSecValidationRequired}
                onCheckedChange={(checked) =>
                  update("dnsSecValidationRequired", checked)
                }
              />
            </div>
            <div className="flex items-center justify-between gap-2">
              <Label htmlFor="dnsSecQueryIpsecRequired">
                {t("nrpt.fields.dnsSecQueryIpsecRequired")}
              </Label>
              <Switch
                id="dnsSecQueryIpsecRequired"
                checked={form.dnsSecQueryIpsecRequired}
                onCheckedChange={(checked) =>
                  update("dnsSecQueryIpsecRequired", checked)
                }
              />
            </div>
            <div className="flex flex-col gap-1.5 sm:col-span-2">
              <Label htmlFor="dnsSecQueryIpsecEncryption">
                {t("nrpt.fields.dnsSecQueryIpsecEncryption")}
              </Label>
              <Input
                id="dnsSecQueryIpsecEncryption"
                value={form.dnsSecQueryIpsecEncryption}
                onChange={(e) =>
                  update("dnsSecQueryIpsecEncryption", e.currentTarget.value)
                }
              />
            </div>

            <div className="flex items-center justify-between gap-2 sm:col-span-2">
              <Label htmlFor="directAccessEnabled">
                {t("nrpt.fields.directAccessEnabled")}
              </Label>
              <Switch
                id="directAccessEnabled"
                checked={form.directAccessEnabled}
                onCheckedChange={(checked) =>
                  update("directAccessEnabled", checked)
                }
              />
            </div>
            <div className="flex flex-col gap-1.5 sm:col-span-2">
              <Label htmlFor="directAccessDnsServers">
                {t("nrpt.fields.directAccessDnsServers")}
              </Label>
              <Textarea
                id="directAccessDnsServers"
                placeholder={t("nrpt.dnsServersPlaceholder")}
                value={form.directAccessDnsServers}
                onChange={(e) =>
                  update("directAccessDnsServers", e.currentTarget.value)
                }
              />
            </div>
            <div className="flex flex-col gap-1.5">
              <Label htmlFor="directAccessProxyName">
                {t("nrpt.fields.directAccessProxyName")}
              </Label>
              <Input
                id="directAccessProxyName"
                value={form.directAccessProxyName}
                onChange={(e) =>
                  update("directAccessProxyName", e.currentTarget.value)
                }
              />
            </div>
            <div className="flex flex-col gap-1.5">
              <Label>{t("nrpt.fields.directAccessProxyType")}</Label>
              <Select
                value={form.directAccessProxyType}
                onValueChange={(value) =>
                  update("directAccessProxyType", value as string)
                }
              >
                <SelectTrigger className="w-full">
                  <SelectValue />
                </SelectTrigger>
                <SelectContent>
                  {DA_PROXY_TYPE_OPTIONS.map((option) => (
                    <SelectItem key={option} value={option}>
                      {option}
                    </SelectItem>
                  ))}
                </SelectContent>
              </Select>
            </div>
            <div className="flex items-center justify-between gap-2">
              <Label htmlFor="directAccessQueryIpsecRequired">
                {t("nrpt.fields.directAccessQueryIpsecRequired")}
              </Label>
              <Switch
                id="directAccessQueryIpsecRequired"
                checked={form.directAccessQueryIpsecRequired}
                onCheckedChange={(checked) =>
                  update("directAccessQueryIpsecRequired", checked)
                }
              />
            </div>
            <div className="flex flex-col gap-1.5 sm:col-span-2">
              <Label htmlFor="directAccessQueryIpsecEncryption">
                {t("nrpt.fields.directAccessQueryIpsecEncryption")}
              </Label>
              <Input
                id="directAccessQueryIpsecEncryption"
                value={form.directAccessQueryIpsecEncryption}
                onChange={(e) =>
                  update(
                    "directAccessQueryIpsecEncryption",
                    e.currentTarget.value,
                  )
                }
              />
            </div>
          </div>
        </CollapsibleDetails>

        <div>
          <Button onClick={addRule} disabled={!canAdd}>
            <Plus />
            {t("nrpt.addRule")}
          </Button>
        </div>
      </CardContent>
    </Card>
  );
}

export function NrptRulesPage() {
  const { t } = useTranslation();
  const systemRules = useSystemNrptRules();
  const { isAdministrator } = useIsAdministrator();
  const [rules, setRules] = useState<NrptRule[]>([]);

  function addRule(rule: NrptRule) {
    setRules((prev) => [...prev, rule]);
  }

  function removeRule(id: string) {
    setRules((prev) => prev.filter((rule) => rule.id !== id));
  }

  return (
    <div className="flex flex-col gap-6">
      <div>
        <h1 className="text-lg font-semibold">{t("nrpt.title")}</h1>
        <p className="text-sm text-muted-foreground">
          <Trans
            i18nKey="nrpt.subtitle"
            components={{ 1: <code className="text-xs" /> }}
          />
        </p>
      </div>

      <div className="flex flex-col gap-3">
        <div className="flex items-center justify-between">
          <h2 className="text-sm font-medium text-muted-foreground">
            {t("nrpt.currentRules")}
          </h2>
          <div className="flex items-center gap-2">
            {systemRules.updatedAt && (
              <span className="text-xs text-muted-foreground">
                {t("common.updatedAgo", {
                  time: formatRelativeTime(t, systemRules.updatedAt),
                })}
              </span>
            )}
            <Button
              variant="ghost"
              size="icon-sm"
              onClick={systemRules.refresh}
              disabled={systemRules.status === "loading"}
            >
              <RefreshCw
                className={
                  systemRules.status === "loading" ? "animate-spin" : ""
                }
              />
            </Button>
          </div>
        </div>
        {systemRules.status === "loading" && systemRules.rules.length === 0 && (
          <p className="flex items-center gap-2 text-sm text-muted-foreground">
            <Loader2 className="size-4 animate-spin" />
            {t("nrpt.reading")}
          </p>
        )}
        {systemRules.status === "error" && systemRules.rules.length === 0 && (
          <p className="text-sm text-destructive">
            {t("nrpt.couldntRead", { error: systemRules.error })}
          </p>
        )}
        {systemRules.status === "ready" && systemRules.rules.length === 0 && (
          <p className="text-sm text-muted-foreground">
            {t("nrpt.noneConfigured")}
          </p>
        )}
        {systemRules.rules.map((rule) => (
          <SystemRuleCard
            key={rule.name}
            rule={rule}
            t={t}
            isAdministrator={isAdministrator}
            onRemoved={systemRules.refresh}
          />
        ))}
      </div>

      <NewRuleForm t={t} onAdd={addRule} />

      <div className="flex flex-col gap-3">
        <h2 className="text-sm font-medium text-muted-foreground">
          {t("nrpt.pendingRules")}
        </h2>
        {rules.length === 0 ? (
          <p className="text-sm text-muted-foreground">
            {t("nrpt.noRulesYet")}
          </p>
        ) : (
          rules.map((rule) => (
            <Card key={rule.id}>
              <CardContent className="flex items-start justify-between gap-4">
                <div className="flex flex-col gap-2">
                  <span className="font-medium">{rule.namespace}</span>
                  <div className="flex flex-wrap gap-1.5">
                    {rule.servers.map((server) => (
                      <Badge key={server} variant="secondary">
                        {server}
                      </Badge>
                    ))}
                  </div>
                  {rule.comment && (
                    <span className="text-sm text-muted-foreground">
                      {rule.comment}
                    </span>
                  )}
                </div>
                <Button
                  variant="ghost"
                  size="icon"
                  onClick={() => removeRule(rule.id)}
                >
                  <Trash2 />
                </Button>
              </CardContent>
            </Card>
          ))
        )}
      </div>
    </div>
  );
}
