import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { Loader2 } from "lucide-react";
import { useTranslation } from "react-i18next";
import { toast } from "sonner";

import { Button } from "@/components/ui/button";
import { Checkbox } from "@/components/ui/checkbox";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
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

export interface RulePrefill {
  name?: string;
  ports?: string;
  protocol?: "TCP" | "UDP";
  program?: string | null;
}

const PROFILES = ["Domain", "Private", "Public"] as const;
type Profile = (typeof PROFILES)[number];

function errorMessage(err: unknown) {
  return err instanceof Error ? err.message : String(err);
}

/**
 * Creates a firewall rule for a port (and optionally one program). Used from
 * the Firewall page and from a row on the Ports page. Rules made here are
 * tagged as this app's, so they can be found and removed again later.
 */
export function FirewallRuleDialog({
  open,
  prefill,
  onClose,
  onCreated,
}: {
  open: boolean;
  prefill: RulePrefill;
  onClose: () => void;
  onCreated: () => void;
}) {
  const { t } = useTranslation();
  const [name, setName] = useState("");
  const [direction, setDirection] = useState("Inbound");
  const [action, setAction] = useState("Allow");
  const [protocol, setProtocol] = useState("TCP");
  const [ports, setPorts] = useState("");
  const [program, setProgram] = useState("");
  const [profiles, setProfiles] = useState<Profile[]>(["Private"]);
  const [remote, setRemote] = useState("LocalSubnet");
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState<string | null>(null);

  // Start from the prefill each time the dialog opens.
  useEffect(() => {
    if (!open) return;
    setName(prefill.name ?? "");
    setDirection("Inbound");
    setAction("Allow");
    setProtocol(prefill.protocol ?? "TCP");
    setPorts(prefill.ports ?? "");
    setProgram(prefill.program ?? "");
    setProfiles(["Private"]);
    setRemote("LocalSubnet");
    setError(null);
    // Re-read the prefill only when the dialog is (re)opened.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [open]);

  function toggleProfile(profile: Profile, on: boolean) {
    setProfiles((prev) => (on ? [...prev, profile] : prev.filter((p) => p !== profile)));
  }

  async function create() {
    setSaving(true);
    setError(null);
    try {
      await invoke("create_firewall_rule", {
        spec: {
          name,
          direction,
          action,
          protocol,
          ports,
          program: program.trim() || null,
          profiles,
          remote,
        },
      });
      toast.success(t("firewall.rule.created", { name: name.trim() }));
      onCreated();
      onClose();
    } catch (err) {
      setError(errorMessage(err));
    } finally {
      setSaving(false);
    }
  }

  const canCreate = name.trim().length > 0 && ports.trim().length > 0 && profiles.length > 0 && !saving;
  const opensToEveryone = action === "Allow" && direction === "Inbound" && remote === "Any";

  const select = (
    id: string,
    value: string,
    onChange: (v: string) => void,
    options: [string, string][],
  ) => (
    <Select value={value} onValueChange={(v) => onChange(v as string)} disabled={saving}>
      <SelectTrigger id={id} size="sm" className="w-full">
        <SelectValue>{(v: string) => options.find(([key]) => key === v)?.[1] ?? v}</SelectValue>
      </SelectTrigger>
      <SelectContent>
        {options.map(([key, label]) => (
          <SelectItem key={key} value={key}>
            {label}
          </SelectItem>
        ))}
      </SelectContent>
    </Select>
  );

  return (
    <Dialog open={open} onOpenChange={(next) => !next && !saving && onClose()}>
      <DialogContent className="sm:max-w-xl">
        <DialogHeader>
          <DialogTitle>{t("firewall.rule.title")}</DialogTitle>
          <DialogDescription>{t("firewall.rule.description")}</DialogDescription>
        </DialogHeader>

        <div className="flex flex-col gap-4">
          <div className="flex flex-col gap-1.5">
            <Label htmlFor="ruleName">{t("firewall.rule.nameLabel")}</Label>
            <Input id="ruleName" value={name} onChange={(e) => setName(e.currentTarget.value)} disabled={saving} />
          </div>
          <div className="grid gap-4 sm:grid-cols-3">
            <div className="flex flex-col gap-1.5">
              <Label htmlFor="ruleAction">{t("firewall.columns.action")}</Label>
              {select("ruleAction", action, setAction, [
                ["Allow", t("firewall.allow")],
                ["Block", t("firewall.block")],
              ])}
            </div>
            <div className="flex flex-col gap-1.5">
              <Label htmlFor="ruleDirection">{t("firewall.columns.direction")}</Label>
              {select("ruleDirection", direction, setDirection, [
                ["Inbound", t("firewall.inbound")],
                ["Outbound", t("firewall.outbound")],
              ])}
            </div>
            <div className="flex flex-col gap-1.5">
              <Label htmlFor="ruleProtocol">{t("firewall.columns.protocol")}</Label>
              {select("ruleProtocol", protocol, setProtocol, [
                ["TCP", "TCP"],
                ["UDP", "UDP"],
              ])}
            </div>
          </div>
          <div className="flex flex-col gap-1.5">
            <Label htmlFor="rulePorts">{t("firewall.rule.portsLabel")}</Label>
            <Input
              id="rulePorts"
              value={ports}
              onChange={(e) => setPorts(e.currentTarget.value)}
              placeholder="3000, 8000-8100"
              className="font-mono"
              disabled={saving}
            />
            <p className="text-xs text-muted-foreground">
              {direction === "Inbound" ? t("firewall.rule.portsHintInbound") : t("firewall.rule.portsHintOutbound")}
            </p>
          </div>
          <div className="flex flex-col gap-1.5">
            <Label htmlFor="ruleProgram">{t("firewall.rule.programLabel")}</Label>
            <Input
              id="ruleProgram"
              value={program}
              onChange={(e) => setProgram(e.currentTarget.value)}
              placeholder="C:\\Tools\\app.exe"
              className="font-mono"
              disabled={saving}
            />
            <p className="text-xs text-muted-foreground">{t("firewall.rule.programHint")}</p>
          </div>
          <div className="grid gap-4 sm:grid-cols-2">
            <div className="flex flex-col gap-1.5">
              <Label>{t("firewall.rule.profilesLabel")}</Label>
              <div className="flex flex-wrap gap-3">
                {PROFILES.map((profile) => (
                  <div key={profile} className="flex items-center gap-1.5">
                    <Checkbox
                      id={`ruleProfile${profile}`}
                      checked={profiles.includes(profile)}
                      onCheckedChange={(c) => toggleProfile(profile, c === true)}
                      disabled={saving}
                    />
                    <Label htmlFor={`ruleProfile${profile}`}>{profile}</Label>
                  </div>
                ))}
              </div>
            </div>
            <div className="flex flex-col gap-1.5">
              <Label htmlFor="ruleRemote">{t("firewall.rule.remoteLabel")}</Label>
              {select("ruleRemote", remote, setRemote, [
                ["LocalSubnet", t("firewall.rule.localSubnet")],
                ["Any", t("firewall.rule.anyAddress")],
              ])}
            </div>
          </div>
          {opensToEveryone && (
            <p className="rounded-lg border border-amber-500/40 bg-amber-500/5 p-2 text-sm text-amber-700 dark:text-amber-400">
              {t("firewall.rule.openWarning")}
            </p>
          )}
          <p className="text-xs text-muted-foreground">{t("firewall.rule.adminNote")}</p>
          {error && <p className="text-sm text-destructive">{error}</p>}
        </div>

        <DialogFooter>
          <Button variant="outline" onClick={onClose} disabled={saving}>
            {t("firewall.rule.cancel")}
          </Button>
          <Button onClick={create} disabled={!canCreate}>
            {saving && <Loader2 className="animate-spin" />}
            {saving ? t("firewall.rule.creating") : t("firewall.rule.create")}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
