import { useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { Loader2, Pencil, Plus, RefreshCw, Trash2 } from "lucide-react";
import { useTranslation } from "react-i18next";
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
import { Textarea } from "@/components/ui/textarea";
import { CollapsibleDetails } from "@/components/collapsible-details";
import { useSshConfig, type SshHost } from "@/features/ssh/use-ssh-config";
import { formatRelativeTime } from "@/lib/relative-time";

interface HostFormState {
  patterns: string;
  hostName: string;
  user: string;
  port: string;
  identityFile: string;
  proxyJump: string;
}

function emptyForm(): HostFormState {
  return {
    patterns: "",
    hostName: "",
    user: "",
    port: "",
    identityFile: "",
    proxyJump: "",
  };
}

function formFromHost(host: SshHost): HostFormState {
  return {
    patterns: host.patterns.join(" "),
    hostName: host.hostName ?? "",
    user: host.user ?? "",
    port: host.port ?? "",
    identityFile: host.identityFile ?? "",
    proxyJump: host.proxyJump ?? "",
  };
}

function toInput(form: HostFormState) {
  return {
    patterns: form.patterns.trim(),
    hostName: form.hostName.trim() || null,
    user: form.user.trim() || null,
    port: form.port.trim() || null,
    identityFile: form.identityFile.trim() || null,
    proxyJump: form.proxyJump.trim() || null,
  };
}

function HostFields({
  idPrefix,
  form,
  onChange,
  t,
}: {
  idPrefix: string;
  form: HostFormState;
  onChange: (key: keyof HostFormState, value: string) => void;
  t: TFunction;
}) {
  const fields: { key: keyof HostFormState; label: string; placeholder: string }[] = [
    { key: "patterns", label: "aliasLabel", placeholder: "aliasPlaceholder" },
    { key: "hostName", label: "hostNameLabel", placeholder: "hostNamePlaceholder" },
    { key: "user", label: "userLabel", placeholder: "userPlaceholder" },
    { key: "port", label: "portLabel", placeholder: "portPlaceholder" },
    { key: "identityFile", label: "identityFileLabel", placeholder: "identityFilePlaceholder" },
    { key: "proxyJump", label: "proxyJumpLabel", placeholder: "proxyJumpPlaceholder" },
  ];
  return (
    <div className="grid gap-4 sm:grid-cols-2">
      {fields.map((f) => (
        <div key={f.key} className="flex flex-col gap-1.5">
          <Label htmlFor={`${idPrefix}-${f.key}`}>{t(`ssh.${f.label}`)}</Label>
          <Input
            id={`${idPrefix}-${f.key}`}
            placeholder={t(`ssh.${f.placeholder}`)}
            value={form[f.key]}
            onChange={(e) => onChange(f.key, e.currentTarget.value)}
          />
        </div>
      ))}
    </div>
  );
}

function errorMessage(err: unknown) {
  return err instanceof Error ? err.message : String(err);
}

function EditHostDialog({
  host,
  t,
  onSaved,
}: {
  host: SshHost;
  t: TFunction;
  onSaved: () => void;
}) {
  const [open, setOpen] = useState(false);
  const [form, setForm] = useState<HostFormState>(() => formFromHost(host));
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState<string | null>(null);

  function handleOpenChange(next: boolean) {
    if (saving) return;
    setOpen(next);
    if (next) {
      setForm(formFromHost(host));
      setError(null);
    }
  }

  async function save() {
    setSaving(true);
    setError(null);
    try {
      await invoke("update_ssh_host", {
        lineNumber: host.lineNumber,
        originalPatterns: host.patterns.join(" "),
        input: toInput(form),
      });
      setOpen(false);
      toast.success(t("ssh.edit.success", { alias: form.patterns.trim() }));
      onSaved();
    } catch (err) {
      setError(errorMessage(err));
    } finally {
      setSaving(false);
    }
  }

  return (
    <Dialog open={open} onOpenChange={handleOpenChange}>
      <DialogTrigger
        render={
          <Button variant="ghost" size="icon-sm" aria-label={t("ssh.edit.button")} />
        }
      >
        <Pencil />
      </DialogTrigger>
      <DialogContent>
        <DialogHeader>
          <DialogTitle>{t("ssh.edit.title")}</DialogTitle>
          <DialogDescription>{t("ssh.edit.description")}</DialogDescription>
        </DialogHeader>
        <HostFields
          idPrefix={`ssh-edit-${host.lineNumber}`}
          form={form}
          onChange={(key, value) => setForm((prev) => ({ ...prev, [key]: value }))}
          t={t}
        />
        {error && <p className="text-sm text-destructive">{error}</p>}
        <DialogFooter>
          <Button variant="outline" onClick={() => handleOpenChange(false)} disabled={saving}>
            {t("ssh.cancel")}
          </Button>
          <Button onClick={save} disabled={saving || form.patterns.trim().length === 0}>
            {saving && <Loader2 className="animate-spin" />}
            {saving ? t("ssh.saving") : t("ssh.edit.confirm")}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}

function RemoveHostDialog({
  host,
  t,
  onRemoved,
}: {
  host: SshHost;
  t: TFunction;
  onRemoved: () => void;
}) {
  const [open, setOpen] = useState(false);
  const [removing, setRemoving] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const alias = host.patterns.join(" ");

  function handleOpenChange(next: boolean) {
    if (removing) return;
    setOpen(next);
    if (!next) setError(null);
  }

  async function confirmRemove() {
    setRemoving(true);
    setError(null);
    try {
      await invoke("remove_ssh_host", {
        lineNumber: host.lineNumber,
        originalPatterns: alias,
      });
      setOpen(false);
      toast.success(t("ssh.remove.success", { alias }));
      onRemoved();
    } catch (err) {
      setError(errorMessage(err));
    } finally {
      setRemoving(false);
    }
  }

  return (
    <Dialog open={open} onOpenChange={handleOpenChange}>
      <DialogTrigger
        render={
          <Button variant="ghost" size="icon-sm" aria-label={t("ssh.remove.button")} />
        }
      >
        <Trash2 />
      </DialogTrigger>
      <DialogContent>
        <DialogHeader>
          <DialogTitle>{t("ssh.remove.title")}</DialogTitle>
          <DialogDescription>{t("ssh.remove.description", { alias })}</DialogDescription>
        </DialogHeader>
        {error && <p className="text-sm text-destructive">{error}</p>}
        <DialogFooter>
          <Button variant="outline" onClick={() => handleOpenChange(false)} disabled={removing}>
            {t("ssh.cancel")}
          </Button>
          <Button variant="destructive" onClick={confirmRemove} disabled={removing}>
            {removing && <Loader2 className="animate-spin" />}
            {removing ? t("ssh.remove.removing") : t("ssh.remove.confirm")}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}

const GRID = "grid-cols-[10rem_1fr_8rem_5rem_5rem]";

function HostTable({
  hosts,
  t,
  onChanged,
}: {
  hosts: SshHost[];
  t: TFunction;
  onChanged: () => void;
}) {
  return (
    <div className="overflow-x-auto rounded-lg border">
      <div className="min-w-[42rem]">
        <div
          className={`grid ${GRID} gap-2 border-b bg-muted/50 px-3 py-2 text-xs font-medium text-muted-foreground`}
        >
          <span>{t("ssh.columns.alias")}</span>
          <span>{t("ssh.columns.target")}</span>
          <span>{t("ssh.columns.identity")}</span>
          <span>{t("ssh.columns.port")}</span>
          <span />
        </div>
        {hosts.map((host) => (
          <div
            key={host.lineNumber}
            className={`grid ${GRID} items-center gap-2 border-b px-3 py-2 text-sm last:border-b-0`}
          >
            <div className="flex flex-wrap gap-1">
              {host.patterns.map((p) => (
                <Badge key={p} variant="secondary">
                  {p}
                </Badge>
              ))}
            </div>
            <div className="min-w-0 break-all text-muted-foreground">
              {host.user ? `${host.user}@` : ""}
              {host.hostName ?? ""}
              {host.proxyJump && (
                <span className="ml-2 text-xs">
                  {t("ssh.viaJump", { host: host.proxyJump })}
                </span>
              )}
              {host.otherOptions > 0 && (
                <span className="ml-2 text-xs">
                  {t("ssh.otherOptions", { count: host.otherOptions })}
                </span>
              )}
            </div>
            <span className="truncate text-xs text-muted-foreground" title={host.identityFile ?? ""}>
              {host.identityFile ?? ""}
            </span>
            <span className="text-muted-foreground">{host.port ?? ""}</span>
            <div className="flex justify-end">
              <EditHostDialog host={host} t={t} onSaved={onChanged} />
              <RemoveHostDialog host={host} t={t} onRemoved={onChanged} />
            </div>
          </div>
        ))}
      </div>
    </div>
  );
}

function AddHostForm({ t, onAdded }: { t: TFunction; onAdded: () => void }) {
  const [form, setForm] = useState<HostFormState>(emptyForm);
  const [adding, setAdding] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const canAdd = form.patterns.trim().length > 0 && !adding;

  async function addHost() {
    if (!canAdd) return;
    setAdding(true);
    setError(null);
    try {
      await invoke("add_ssh_host", { input: toInput(form) });
      toast.success(t("ssh.addSuccess", { alias: form.patterns.trim() }));
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
        <CardTitle>{t("ssh.newHost")}</CardTitle>
        <CardDescription>{t("ssh.newHostDescription")}</CardDescription>
      </CardHeader>
      <CardContent className="flex flex-col gap-4">
        <HostFields
          idPrefix="ssh-new"
          form={form}
          onChange={(key, value) => setForm((prev) => ({ ...prev, [key]: value }))}
          t={t}
        />
        {error && <p className="text-sm text-destructive">{error}</p>}
        <div>
          <Button onClick={addHost} disabled={!canAdd}>
            {adding ? <Loader2 className="animate-spin" /> : <Plus />}
            {adding ? t("ssh.adding") : t("ssh.addHost")}
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
      await invoke("set_ssh_config_raw", { content: value });
      toast.success(t("ssh.rawSaveSuccess"));
      onSaved();
    } catch (err) {
      setError(errorMessage(err));
    } finally {
      setSaving(false);
    }
  }

  return (
    <CollapsibleDetails label={t("ssh.rawEditorLabel")}>
      <div className="mt-3 flex flex-col gap-2">
        <p className="text-sm text-muted-foreground">{t("ssh.rawEditorDescription")}</p>
        <Textarea
          value={value}
          onChange={(e) => setValue(e.currentTarget.value)}
          className="min-h-64 font-mono text-xs"
          spellCheck={false}
        />
        {error && <p className="text-sm text-destructive">{error}</p>}
        <div>
          <Button variant="outline" onClick={save} disabled={saving}>
            {saving && <Loader2 className="animate-spin" />}
            {saving ? t("ssh.saving") : t("ssh.saveRaw")}
          </Button>
        </div>
      </div>
    </CollapsibleDetails>
  );
}

export function SshPage() {
  const { t } = useTranslation();
  const ssh = useSshConfig();

  return (
    <div className="flex flex-col gap-6">
      <div>
        <h1 className="text-lg font-semibold">{t("ssh.title")}</h1>
        <p className="text-sm text-muted-foreground">{t("ssh.subtitle")}</p>
      </div>

      <div className="flex flex-col gap-3">
        <div className="flex items-center justify-between">
          <div className="flex items-center gap-2">
            {ssh.updatedAt && (
              <span className="text-xs text-muted-foreground">
                {t("common.updatedAgo", {
                  time: formatRelativeTime(t, ssh.updatedAt),
                })}
              </span>
            )}
          </div>
          <Button
            variant="ghost"
            size="icon-sm"
            onClick={ssh.refresh}
            disabled={ssh.status === "loading"}
          >
            <RefreshCw className={ssh.status === "loading" ? "animate-spin" : ""} />
          </Button>
        </div>

        {ssh.status === "loading" && ssh.hosts.length === 0 && (
          <p className="flex items-center gap-2 text-sm text-muted-foreground">
            <Loader2 className="size-4 animate-spin" />
            {t("ssh.reading")}
          </p>
        )}
        {ssh.status === "error" && ssh.hosts.length === 0 && (
          <p className="text-sm text-destructive">
            {t("ssh.couldntRead", { error: ssh.error })}
          </p>
        )}
        {ssh.status === "ready" && ssh.hosts.length === 0 && (
          <p className="text-sm text-muted-foreground">{t("ssh.noneConfigured")}</p>
        )}
        {ssh.hosts.length > 0 && (
          <HostTable hosts={ssh.hosts} t={t} onChanged={ssh.refresh} />
        )}
      </div>

      <AddHostForm t={t} onAdded={ssh.refresh} />

      <Card>
        <CardContent>
          <RawEditor key={ssh.raw} raw={ssh.raw} t={t} onSaved={ssh.refresh} />
        </CardContent>
      </Card>
    </div>
  );
}
