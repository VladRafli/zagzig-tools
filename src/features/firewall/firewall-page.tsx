import { useMemo, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { Loader2, RefreshCw, Search } from "lucide-react";
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
import { useFirewall, type FirewallRule } from "@/features/firewall/use-firewall";
import { formatRelativeTime } from "@/lib/relative-time";
import { useIsAdministrator } from "@/lib/use-is-administrator";

const PAGE_SIZE = 150;

type DirectionFilter = "all" | "Inbound" | "Outbound";
type ActionFilter = "all" | "Allow" | "Block";
type EnabledFilter = "all" | "enabled" | "disabled";

const GRID = "grid-cols-[2.5rem_1.6fr_5.5rem_4.5rem_4.5rem_6rem_1fr]";

function RuleRow({
  rule,
  t,
  isAdministrator,
  onChanged,
}: {
  rule: FirewallRule;
  t: TFunction;
  isAdministrator: boolean;
  onChanged: () => void;
}) {
  const [busy, setBusy] = useState(false);

  async function toggle(next: boolean) {
    setBusy(true);
    try {
      await invoke("set_firewall_rule_enabled", { name: rule.name, enabled: next });
      toast.success(
        t(next ? "firewall.enabledSuccess" : "firewall.disabledSuccess", {
          name: rule.displayName,
        }),
      );
      onChanged();
    } catch (err) {
      toast.error(
        t("firewall.toggleError", {
          error: err instanceof Error ? err.message : String(err),
        }),
      );
    } finally {
      setBusy(false);
    }
  }

  return (
    <div
      className={`grid ${GRID} items-center gap-2 border-b px-3 py-2 text-sm last:border-b-0 ${
        rule.enabled ? "" : "text-muted-foreground"
      }`}
    >
      <AdminRequiredTooltip locked={!isAdministrator}>
        <Switch
          checked={rule.enabled}
          disabled={!isAdministrator || busy}
          onCheckedChange={toggle}
        />
      </AdminRequiredTooltip>
      <div className="min-w-0">
        <div className="break-words">{rule.displayName}</div>
        {(rule.program || rule.group) && (
          <div className="truncate text-xs text-muted-foreground" title={rule.program ?? rule.group ?? ""}>
            {rule.program ?? rule.group}
          </div>
        )}
      </div>
      <Badge variant="outline" className="w-fit">
        {rule.direction === "Inbound" ? t("firewall.inbound") : t("firewall.outbound")}
      </Badge>
      <Badge variant={rule.action === "Block" ? "destructive" : "secondary"} className="w-fit">
        {rule.action === "Block" ? t("firewall.block") : t("firewall.allow")}
      </Badge>
      <span className="text-xs">{rule.protocol}</span>
      <span className="break-all text-xs">{rule.localPort}</span>
      <span className="break-words text-xs">{rule.profile}</span>
    </div>
  );
}

export function FirewallPage() {
  const { t } = useTranslation();
  const { profiles, rules, status, error, updatedAt, refresh } = useFirewall();
  const { isAdministrator } = useIsAdministrator();

  const [query, setQuery] = useState("");
  const [direction, setDirection] = useState<DirectionFilter>("all");
  const [action, setAction] = useState<ActionFilter>("all");
  const [enabled, setEnabled] = useState<EnabledFilter>("all");
  const [limit, setLimit] = useState(PAGE_SIZE);

  const filtered = useMemo(() => {
    const needle = query.trim().toLowerCase();
    return rules.filter(
      (r) =>
        (direction === "all" || r.direction === direction) &&
        (action === "all" || r.action === action) &&
        (enabled === "all" || (enabled === "enabled") === r.enabled) &&
        (!needle ||
          [r.displayName, r.program, r.group, r.protocol, r.localPort, r.remotePort, r.profile].some(
            (f) => f?.toLowerCase().includes(needle),
          )),
    );
  }, [rules, query, direction, action, enabled]);

  const filterSelect = <V extends string>(
    value: V,
    onChange: (v: V) => void,
    options: V[],
    labelFor: (v: V) => string,
    width: string,
    label: string,
  ) => (
    <Select
      value={value}
      onValueChange={(v) => {
        onChange(v as V);
        setLimit(PAGE_SIZE);
      }}
    >
      <SelectTrigger size="sm" className={width} aria-label={label}>
        <SelectValue>{(v: V) => labelFor(v)}</SelectValue>
      </SelectTrigger>
      <SelectContent>
        {options.map((o) => (
          <SelectItem key={o} value={o}>
            {labelFor(o)}
          </SelectItem>
        ))}
      </SelectContent>
    </Select>
  );

  return (
    <div className="flex flex-col gap-6">
      <div>
        <h1 className="text-lg font-semibold">{t("firewall.title")}</h1>
        <p className="text-sm text-muted-foreground">{t("firewall.subtitle")}</p>
      </div>

      {profiles.length > 0 && (
        <div className="flex flex-wrap items-center gap-2 text-sm">
          <span className="text-muted-foreground">{t("firewall.profiles")}</span>
          {profiles.map((p) => (
            <Badge key={p.name} variant={p.enabled ? "default" : "outline"}>
              {p.name}: {p.enabled ? t("firewall.on") : t("firewall.off")}
            </Badge>
          ))}
        </div>
      )}

      <div className="flex flex-col gap-3">
        <div className="flex flex-wrap items-center gap-2">
          <div className="relative min-w-48 flex-1">
            <Search className="pointer-events-none absolute top-1/2 left-2.5 size-4 -translate-y-1/2 text-muted-foreground" />
            <Input
              className="pl-8"
              placeholder={t("firewall.searchPlaceholder")}
              value={query}
              onChange={(e) => {
                setQuery(e.currentTarget.value);
                setLimit(PAGE_SIZE);
              }}
            />
          </div>
          {filterSelect<DirectionFilter>(
            direction,
            setDirection,
            ["all", "Inbound", "Outbound"],
            (v) => (v === "all" ? t("firewall.anyDirection") : v === "Inbound" ? t("firewall.inbound") : t("firewall.outbound")),
            "w-36",
            t("firewall.columns.direction"),
          )}
          {filterSelect<ActionFilter>(
            action,
            setAction,
            ["all", "Allow", "Block"],
            (v) => (v === "all" ? t("firewall.anyAction") : v === "Allow" ? t("firewall.allow") : t("firewall.block")),
            "w-32",
            t("firewall.columns.action"),
          )}
          {filterSelect<EnabledFilter>(
            enabled,
            setEnabled,
            ["all", "enabled", "disabled"],
            (v) => t(`firewall.enabledFilter.${v}`),
            "w-36",
            t("firewall.columns.on"),
          )}
          <div className="ml-auto flex items-center gap-2">
            {updatedAt && (
              <span className="text-xs text-muted-foreground">
                {t("common.updatedAgo", { time: formatRelativeTime(t, updatedAt) })}
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

        {status === "loading" && rules.length === 0 && (
          <p className="flex items-center gap-2 text-sm text-muted-foreground">
            <Loader2 className="size-4 animate-spin" />
            {t("firewall.reading")}
          </p>
        )}
        {status === "error" && rules.length === 0 && (
          <p className="text-sm text-destructive">{t("firewall.couldntRead", { error })}</p>
        )}
        {status === "ready" && filtered.length === 0 && (
          <p className="text-sm text-muted-foreground">{t("firewall.noMatches")}</p>
        )}
        {filtered.length > 0 && (
          <>
            <p className="text-xs text-muted-foreground">
              {t("firewall.count", { shown: Math.min(limit, filtered.length), total: filtered.length })}
            </p>
            <div className="overflow-x-auto rounded-lg border">
              <div className="min-w-[56rem]">
                <div
                  className={`grid ${GRID} gap-2 border-b bg-muted/50 px-3 py-2 text-xs font-medium text-muted-foreground`}
                >
                  <span>{t("firewall.columns.on")}</span>
                  <span>{t("firewall.columns.name")}</span>
                  <span>{t("firewall.columns.direction")}</span>
                  <span>{t("firewall.columns.action")}</span>
                  <span>{t("firewall.columns.protocol")}</span>
                  <span>{t("firewall.columns.localPort")}</span>
                  <span>{t("firewall.columns.profile")}</span>
                </div>
                {filtered.slice(0, limit).map((rule) => (
                  <RuleRow
                    key={rule.name}
                    rule={rule}
                    t={t}
                    isAdministrator={isAdministrator}
                    onChanged={refresh}
                  />
                ))}
              </div>
            </div>
            {filtered.length > limit && (
              <div>
                <Button variant="outline" size="sm" onClick={() => setLimit((l) => l + PAGE_SIZE)}>
                  {t("firewall.showMore")}
                </Button>
              </div>
            )}
          </>
        )}
      </div>
    </div>
  );
}
