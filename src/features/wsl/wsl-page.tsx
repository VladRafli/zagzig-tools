import { useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { Container, Loader2, Power, RefreshCw, Square, Star, Zap } from "lucide-react";
import { useTranslation } from "react-i18next";
import type { TFunction } from "i18next";
import { toast } from "sonner";

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
import { Checkbox } from "@/components/ui/checkbox";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { Textarea } from "@/components/ui/textarea";
import { CollapsibleDetails } from "@/components/collapsible-details";
import {
  useWsl,
  type WslDistro,
  type WslSettings,
} from "@/features/wsl/use-wsl";
import { formatRelativeTime } from "@/lib/relative-time";

function errorMessage(err: unknown) {
  return err instanceof Error ? err.message : String(err);
}

function ShutdownDialog({
  t,
  onDone,
}: {
  t: TFunction;
  onDone: () => void;
}) {
  const [open, setOpen] = useState(false);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);

  function handleOpenChange(next: boolean) {
    if (busy) return;
    setOpen(next);
    if (!next) setError(null);
  }

  async function confirm() {
    setBusy(true);
    setError(null);
    try {
      await invoke("wsl_shutdown");
      setOpen(false);
      toast.success(t("wsl.shutdown.success"));
      onDone();
    } catch (err) {
      setError(errorMessage(err));
    } finally {
      setBusy(false);
    }
  }

  return (
    <Dialog open={open} onOpenChange={handleOpenChange}>
      <DialogTrigger render={<Button variant="outline" size="sm" />}>
        <Power />
        {t("wsl.shutdown.button")}
      </DialogTrigger>
      <DialogContent>
        <DialogHeader>
          <DialogTitle>{t("wsl.shutdown.title")}</DialogTitle>
          <DialogDescription>
            {t("wsl.shutdown.description")}
          </DialogDescription>
        </DialogHeader>
        {error && <p className="text-sm text-destructive">{error}</p>}
        <DialogFooter>
          <Button variant="outline" onClick={() => handleOpenChange(false)} disabled={busy}>
            {t("wsl.cancel")}
          </Button>
          <Button variant="destructive" onClick={confirm} disabled={busy}>
            {busy && <Loader2 className="animate-spin" />}
            {busy ? t("wsl.shutdown.shuttingDown") : t("wsl.shutdown.confirm")}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}

interface ForceRestartResult {
  killed: string[];
  started: string[];
  failedToStart: string[];
  dockerRestarted: boolean;
  dockerError: string | null;
}

function RestartDockerDialog({
  t,
  onDone,
}: {
  t: TFunction;
  onDone: () => void;
}) {
  const [open, setOpen] = useState(false);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);

  function handleOpenChange(next: boolean) {
    if (busy) return;
    setOpen(next);
    if (!next) setError(null);
  }

  async function confirm() {
    setBusy(true);
    setError(null);
    try {
      await invoke("restart_docker_desktop");
      setOpen(false);
      toast.success(t("wsl.docker.success"));
      onDone();
    } catch (err) {
      setError(errorMessage(err));
    } finally {
      setBusy(false);
    }
  }

  return (
    <Dialog open={open} onOpenChange={handleOpenChange}>
      <DialogTrigger render={<Button variant="outline" size="sm" />}>
        <Container />
        {t("wsl.docker.button")}
      </DialogTrigger>
      <DialogContent>
        <DialogHeader>
          <DialogTitle>{t("wsl.docker.title")}</DialogTitle>
          <DialogDescription>{t("wsl.docker.description")}</DialogDescription>
        </DialogHeader>
        {error && <p className="text-sm text-destructive">{error}</p>}
        <DialogFooter>
          <Button variant="outline" onClick={() => handleOpenChange(false)} disabled={busy}>
            {t("wsl.cancel")}
          </Button>
          <Button variant="destructive" onClick={confirm} disabled={busy}>
            {busy && <Loader2 className="animate-spin" />}
            {busy ? t("wsl.docker.restarting") : t("wsl.docker.confirm")}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}

// For a hung/broken WSL where a plain shutdown doesn't work (e.g. the Docker
// Desktop integration is gone and nothing responds): stops the WSL service,
// kills its leftover processes, then starts WSL again.
function ForceRestartDialog({
  t,
  unresponsive,
  dockerInstalled,
  onDone,
}: {
  t: TFunction;
  unresponsive: boolean;
  dockerInstalled: boolean;
  onDone: () => void;
}) {
  const [open, setOpen] = useState(false);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [restartDocker, setRestartDocker] = useState(true);

  function handleOpenChange(next: boolean) {
    if (busy) return;
    setOpen(next);
    if (!next) setError(null);
  }

  async function confirm() {
    setBusy(true);
    setError(null);
    try {
      const result = await invoke<ForceRestartResult>("wsl_force_restart", {
        restartDocker: dockerInstalled && restartDocker,
      });
      setOpen(false);
      if (result.dockerError) {
        toast.warning(t("wsl.forceRestart.dockerFailed", { error: result.dockerError }));
      } else if (result.failedToStart.length > 0) {
        toast.warning(
          t("wsl.forceRestart.partial", {
            names: result.failedToStart.join(", "),
          }),
        );
      } else {
        toast.success(t("wsl.forceRestart.success"));
      }
      onDone();
    } catch (err) {
      setError(errorMessage(err));
    } finally {
      setBusy(false);
    }
  }

  return (
    <Dialog open={open} onOpenChange={handleOpenChange}>
      <DialogTrigger
        render={
          <Button variant={unresponsive ? "destructive" : "outline"} size="sm" />
        }
      >
        <Zap />
        {t("wsl.forceRestart.button")}
      </DialogTrigger>
      <DialogContent>
        <DialogHeader>
          <DialogTitle>{t("wsl.forceRestart.title")}</DialogTitle>
          <DialogDescription>{t("wsl.forceRestart.description")}</DialogDescription>
        </DialogHeader>
        {dockerInstalled && (
          <div className="flex items-start gap-2">
            <Checkbox
              id="wslRestartDocker"
              checked={restartDocker}
              onCheckedChange={(checked) => setRestartDocker(checked === true)}
              disabled={busy}
            />
            <Label htmlFor="wslRestartDocker" className="leading-snug">
              {t("wsl.forceRestart.alsoDocker")}
            </Label>
          </div>
        )}
        {error && <p className="text-sm text-destructive">{error}</p>}
        <DialogFooter>
          <Button variant="outline" onClick={() => handleOpenChange(false)} disabled={busy}>
            {t("wsl.cancel")}
          </Button>
          <Button variant="destructive" onClick={confirm} disabled={busy}>
            {busy && <Loader2 className="animate-spin" />}
            {busy ? t("wsl.forceRestart.restarting") : t("wsl.forceRestart.confirm")}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}

function DistroRow({
  distro,
  t,
  onChanged,
}: {
  distro: WslDistro;
  t: TFunction;
  onChanged: () => void;
}) {
  const [busy, setBusy] = useState(false);

  async function run(command: string, successKey: string) {
    setBusy(true);
    try {
      await invoke(command, { name: distro.name });
      toast.success(t(successKey, { name: distro.name }));
      onChanged();
    } catch (err) {
      toast.error(t("wsl.actionError", { error: errorMessage(err) }));
    } finally {
      setBusy(false);
    }
  }

  return (
    <div className="grid grid-cols-[1fr_6rem_4rem_auto] items-center gap-2 border-b px-3 py-2 text-sm last:border-b-0">
      <div className="flex items-center gap-2">
        <span className="break-all font-medium">{distro.name}</span>
        {distro.isDefault && <Badge variant="secondary">{t("wsl.default")}</Badge>}
      </div>
      <Badge variant={distro.running ? "default" : "outline"} className="w-fit">
        {distro.running ? t("wsl.running") : t("wsl.stopped")}
      </Badge>
      <span className="text-muted-foreground">WSL {distro.version}</span>
      <div className="flex justify-end gap-1">
        <Button
          variant="ghost"
          size="icon-sm"
          aria-label={t("wsl.setDefault")}
          title={t("wsl.setDefault")}
          disabled={busy || distro.isDefault}
          onClick={() => run("wsl_set_default_distro", "wsl.setDefaultSuccess")}
        >
          <Star />
        </Button>
        <Button
          variant="ghost"
          size="icon-sm"
          aria-label={t("wsl.terminate")}
          title={t("wsl.terminate")}
          disabled={busy || !distro.running}
          onClick={() => run("wsl_terminate_distro", "wsl.terminateSuccess")}
        >
          {busy ? <Loader2 className="animate-spin" /> : <Square />}
        </Button>
      </div>
    </div>
  );
}

const DEFAULT_OPTION = "__default__";

const NETWORKING_MODES = ["NAT", "mirrored", "bridged", "virtioproxy", "none"];
const MEMORY_RECLAIM = ["disabled", "gradual", "dropcache"];
const BOOLEANS = ["true", "false"];

type SettingsForm = Record<keyof WslSettings, string>;

function formFromSettings(s: WslSettings | null): SettingsForm {
  return {
    memory: s?.memory ?? "",
    processors: s?.processors ?? "",
    swap: s?.swap ?? "",
    localhostForwarding: s?.localhostForwarding ?? "",
    networkingMode: s?.networkingMode ?? "",
    autoMemoryReclaim: s?.autoMemoryReclaim ?? "",
    nestedVirtualization: s?.nestedVirtualization ?? "",
  };
}

function ChoiceField({
  id,
  label,
  value,
  options,
  onChange,
  t,
}: {
  id: string;
  label: string;
  value: string;
  options: string[];
  onChange: (value: string) => void;
  t: TFunction;
}) {
  return (
    <div className="flex flex-col gap-1.5">
      <Label htmlFor={id}>{label}</Label>
      <Select
        value={value || DEFAULT_OPTION}
        onValueChange={(v) => onChange(v === DEFAULT_OPTION ? "" : (v as string))}
      >
        <SelectTrigger id={id} size="sm" className="w-full">
          <SelectValue>
            {(v: string) => (v === DEFAULT_OPTION ? t("wsl.config.useDefault") : v)}
          </SelectValue>
        </SelectTrigger>
        <SelectContent>
          <SelectItem value={DEFAULT_OPTION}>{t("wsl.config.useDefault")}</SelectItem>
          {options.map((o) => (
            <SelectItem key={o} value={o}>
              {o}
            </SelectItem>
          ))}
        </SelectContent>
      </Select>
    </div>
  );
}

function ConfigForm({
  settings,
  t,
  onSaved,
}: {
  settings: WslSettings | null;
  t: TFunction;
  onSaved: () => void;
}) {
  const [form, setForm] = useState<SettingsForm>(() => formFromSettings(settings));
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState<string | null>(null);

  function update(key: keyof WslSettings, value: string) {
    setForm((prev) => ({ ...prev, [key]: value }));
  }

  async function save() {
    setSaving(true);
    setError(null);
    try {
      await invoke("set_wsl_settings", {
        settings: Object.fromEntries(
          Object.entries(form).map(([k, v]) => [k, v.trim() || null]),
        ),
      });
      toast.success(t("wsl.config.saveSuccess"));
      onSaved();
    } catch (err) {
      setError(errorMessage(err));
    } finally {
      setSaving(false);
    }
  }

  return (
    <Card>
      <CardHeader>
        <CardTitle>{t("wsl.config.title")}</CardTitle>
        <CardDescription>{t("wsl.config.description")}</CardDescription>
      </CardHeader>
      <CardContent className="flex flex-col gap-4">
        <div className="grid gap-4 sm:grid-cols-2">
          <div className="flex flex-col gap-1.5">
            <Label htmlFor="wslMemory">{t("wsl.config.memory")}</Label>
            <Input
              id="wslMemory"
              placeholder="8GB"
              value={form.memory}
              onChange={(e) => update("memory", e.currentTarget.value)}
            />
          </div>
          <div className="flex flex-col gap-1.5">
            <Label htmlFor="wslProcessors">{t("wsl.config.processors")}</Label>
            <Input
              id="wslProcessors"
              placeholder="4"
              value={form.processors}
              onChange={(e) => update("processors", e.currentTarget.value)}
            />
          </div>
          <div className="flex flex-col gap-1.5">
            <Label htmlFor="wslSwap">{t("wsl.config.swap")}</Label>
            <Input
              id="wslSwap"
              placeholder="4GB"
              value={form.swap}
              onChange={(e) => update("swap", e.currentTarget.value)}
            />
          </div>
          <ChoiceField
            id="wslNetworking"
            label={t("wsl.config.networkingMode")}
            value={form.networkingMode}
            options={NETWORKING_MODES}
            onChange={(v) => update("networkingMode", v)}
            t={t}
          />
          <ChoiceField
            id="wslReclaim"
            label={t("wsl.config.autoMemoryReclaim")}
            value={form.autoMemoryReclaim}
            options={MEMORY_RECLAIM}
            onChange={(v) => update("autoMemoryReclaim", v)}
            t={t}
          />
          <ChoiceField
            id="wslLocalhost"
            label={t("wsl.config.localhostForwarding")}
            value={form.localhostForwarding}
            options={BOOLEANS}
            onChange={(v) => update("localhostForwarding", v)}
            t={t}
          />
          <ChoiceField
            id="wslNested"
            label={t("wsl.config.nestedVirtualization")}
            value={form.nestedVirtualization}
            options={BOOLEANS}
            onChange={(v) => update("nestedVirtualization", v)}
            t={t}
          />
        </div>
        <p className="text-xs text-muted-foreground">{t("wsl.config.restartNote")}</p>
        {error && <p className="text-sm text-destructive">{error}</p>}
        <div>
          <Button onClick={save} disabled={saving}>
            {saving && <Loader2 className="animate-spin" />}
            {saving ? t("wsl.saving") : t("wsl.config.save")}
          </Button>
        </div>
      </CardContent>
    </Card>
  );
}

function RawEditor({
  raw,
  t,
  onSaved,
}: {
  raw: string;
  t: TFunction;
  onSaved: () => void;
}) {
  const [value, setValue] = useState(raw);
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState<string | null>(null);

  async function save() {
    setSaving(true);
    setError(null);
    try {
      await invoke("set_wsl_config_raw", { content: value });
      toast.success(t("wsl.rawSaveSuccess"));
      onSaved();
    } catch (err) {
      setError(errorMessage(err));
    } finally {
      setSaving(false);
    }
  }

  return (
    <CollapsibleDetails label={t("wsl.rawEditorLabel")}>
      <div className="mt-3 flex flex-col gap-2">
        <p className="text-sm text-muted-foreground">{t("wsl.rawEditorDescription")}</p>
        <Textarea
          value={value}
          onChange={(e) => setValue(e.currentTarget.value)}
          className="min-h-48 font-mono text-xs"
          spellCheck={false}
        />
        {error && <p className="text-sm text-destructive">{error}</p>}
        <div>
          <Button variant="outline" onClick={save} disabled={saving}>
            {saving && <Loader2 className="animate-spin" />}
            {saving ? t("wsl.saving") : t("wsl.saveRaw")}
          </Button>
        </div>
      </div>
    </CollapsibleDetails>
  );
}

export function WslPage() {
  const { t } = useTranslation();
  const wsl = useWsl();

  return (
    <div className="flex flex-col gap-6">
      <div>
        <h1 className="text-lg font-semibold">{t("wsl.title")}</h1>
        <p className="text-sm text-muted-foreground">{t("wsl.subtitle")}</p>
      </div>

      <div className="flex flex-col gap-3">
        <div className="flex items-center justify-between">
          <div className="flex items-center gap-2">
            {wsl.updatedAt && (
              <span className="text-xs text-muted-foreground">
                {t("common.updatedAgo", {
                  time: formatRelativeTime(t, wsl.updatedAt),
                })}
              </span>
            )}
          </div>
          <div className="flex items-center gap-1">
            {wsl.installed && wsl.loaded && (
              <>
                {!wsl.unresponsive && <ShutdownDialog t={t} onDone={wsl.refresh} />}
                {wsl.dockerDesktop.installed && (
                  <RestartDockerDialog t={t} onDone={wsl.refresh} />
                )}
                <ForceRestartDialog
                  t={t}
                  unresponsive={wsl.unresponsive}
                  dockerInstalled={wsl.dockerDesktop.installed}
                  onDone={wsl.refresh}
                />
              </>
            )}
            <Button
              variant="ghost"
              size="icon-sm"
              onClick={wsl.refresh}
              disabled={wsl.status === "loading"}
            >
              <RefreshCw className={wsl.status === "loading" ? "animate-spin" : ""} />
            </Button>
          </div>
        </div>

        {wsl.status === "loading" && !wsl.loaded && (
          <p className="flex items-center gap-2 text-sm text-muted-foreground">
            <Loader2 className="size-4 animate-spin" />
            {t("wsl.reading")}
          </p>
        )}
        {wsl.status === "error" && !wsl.loaded && (
          <p className="text-sm text-destructive">
            {t("wsl.couldntRead", { error: wsl.error })}
          </p>
        )}
        {wsl.unresponsive && (
          <p className="text-sm text-destructive">{t("wsl.unresponsive")}</p>
        )}
        {wsl.loaded && !wsl.installed && (
          <p className="text-sm text-muted-foreground">{t("wsl.notInstalled")}</p>
        )}
        {wsl.installed && wsl.loaded && wsl.distros.length === 0 && (
          <p className="text-sm text-muted-foreground">{t("wsl.noDistros")}</p>
        )}
        {wsl.distros.length > 0 && (
          <div className="overflow-x-auto rounded-lg border">
            <div className="min-w-[32rem]">
              {wsl.distros.map((d) => (
                <DistroRow key={d.name} distro={d} t={t} onChanged={wsl.refresh} />
              ))}
            </div>
          </div>
        )}
      </div>

      {wsl.loaded && (
        <>
          <ConfigForm
            key={wsl.rawConfig}
            settings={wsl.settings}
            t={t}
            onSaved={wsl.refresh}
          />
          <Card>
            <CardContent>
              <RawEditor key={wsl.rawConfig} raw={wsl.rawConfig} t={t} onSaved={wsl.refresh} />
            </CardContent>
          </Card>
        </>
      )}
    </div>
  );
}
