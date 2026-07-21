import { useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { Loader2, Lock, Plus, RefreshCw, Trash2 } from "lucide-react";
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
import { Switch } from "@/components/ui/switch";
import { Textarea } from "@/components/ui/textarea";
import { CollapsibleDetails } from "@/components/collapsible-details";
import { useHostsFile, type HostsEntry } from "@/features/hosts/use-hosts-file";
import { formatRelativeTime } from "@/lib/relative-time";
import { useIsAdministrator } from "@/lib/use-is-administrator";
import { AdminRequiredTooltip } from "@/components/admin-required-tooltip";

function RemoveEntryDialog({
  entry,
  t,
  isAdministrator,
  onRemoved,
}: {
  entry: HostsEntry;
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
      await invoke("remove_hosts_entry", { lineNumber: entry.lineNumber });
      setOpen(false);
      toast.success(
        t("hosts.remove.success", { hostnames: entry.hostnames.join(", ") }),
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
              aria-label={t("hosts.remove.button")}
              disabled={!isAdministrator}
            />
          }
        >
          {isAdministrator ? <Trash2 /> : <Lock />}
        </DialogTrigger>
      </AdminRequiredTooltip>
      <DialogContent>
        <DialogHeader>
          <DialogTitle>{t("hosts.remove.title")}</DialogTitle>
          <DialogDescription>
            {t("hosts.remove.description", {
              hostnames: entry.hostnames.join(", "),
              ip: entry.ip,
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
            {t("hosts.cancel")}
          </Button>
          <Button variant="destructive" onClick={confirmRemove} disabled={removing}>
            {removing && <Loader2 className="animate-spin" />}
            {removing ? t("hosts.remove.removing") : t("hosts.remove.confirm")}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}

function EntryRow({
  entry,
  t,
  isAdministrator,
  onChanged,
}: {
  entry: HostsEntry;
  t: TFunction;
  isAdministrator: boolean;
  onChanged: () => void;
}) {
  const [toggling, setToggling] = useState(false);

  async function toggle(next: boolean) {
    setToggling(true);
    try {
      await invoke("set_hosts_entry_enabled", {
        lineNumber: entry.lineNumber,
        enabled: next,
      });
      onChanged();
    } catch (err) {
      toast.error(
        t("hosts.toggleError", {
          error: err instanceof Error ? err.message : String(err),
        }),
      );
    } finally {
      setToggling(false);
    }
  }

  return (
    <div className="grid grid-cols-[2.5rem_9rem_1fr_10rem_2.5rem] items-center gap-2 border-b px-3 py-2 text-sm last:border-b-0">
      <AdminRequiredTooltip locked={!isAdministrator}>
        <Switch
          checked={entry.enabled}
          disabled={!isAdministrator || toggling}
          onCheckedChange={toggle}
        />
      </AdminRequiredTooltip>
      <span className="break-all text-muted-foreground">{entry.ip}</span>
      <div className="flex flex-wrap gap-1">
        {entry.hostnames.map((h) => (
          <Badge key={h} variant={entry.enabled ? "secondary" : "outline"}>
            {h}
          </Badge>
        ))}
      </div>
      <span className="truncate text-xs text-muted-foreground">
        {entry.comment ?? ""}
      </span>
      <div className="flex justify-end">
        <RemoveEntryDialog
          entry={entry}
          t={t}
          isAdministrator={isAdministrator}
          onRemoved={onChanged}
        />
      </div>
    </div>
  );
}

function EntryTable({
  entries,
  t,
  isAdministrator,
  onChanged,
}: {
  entries: HostsEntry[];
  t: TFunction;
  isAdministrator: boolean;
  onChanged: () => void;
}) {
  return (
    <div className="overflow-x-auto rounded-lg border">
      <div className="min-w-[42rem]">
        <div className="grid grid-cols-[2.5rem_9rem_1fr_10rem_2.5rem] gap-2 border-b bg-muted/50 px-3 py-2 text-xs font-medium text-muted-foreground">
          <span>{t("hosts.columns.enabled")}</span>
          <span>{t("hosts.columns.ip")}</span>
          <span>{t("hosts.columns.hostnames")}</span>
          <span>{t("hosts.columns.comment")}</span>
          <span />
        </div>
        {entries.map((entry) => (
          <EntryRow
            key={entry.lineNumber}
            entry={entry}
            t={t}
            isAdministrator={isAdministrator}
            onChanged={onChanged}
          />
        ))}
      </div>
    </div>
  );
}

interface AddEntryFormState {
  ip: string;
  hostnames: string;
  comment: string;
}

function emptyAddEntryForm(): AddEntryFormState {
  return { ip: "", hostnames: "", comment: "" };
}

function AddEntryForm({
  t,
  isAdministrator,
  onAdded,
}: {
  t: TFunction;
  isAdministrator: boolean;
  onAdded: () => void;
}) {
  const [form, setForm] = useState<AddEntryFormState>(emptyAddEntryForm);
  const [adding, setAdding] = useState(false);
  const [error, setError] = useState<string | null>(null);

  function update<K extends keyof AddEntryFormState>(key: K, value: string) {
    setForm((prev) => ({ ...prev, [key]: value }));
  }

  const canAdd =
    isAdministrator &&
    form.ip.trim().length > 0 &&
    form.hostnames.trim().length > 0 &&
    !adding;

  async function addEntry() {
    if (!canAdd) return;
    setAdding(true);
    setError(null);
    try {
      const hostnames = form.hostnames
        .split(/[\s,]+/)
        .map((h) => h.trim())
        .filter(Boolean);
      await invoke("add_hosts_entry", {
        ip: form.ip.trim(),
        hostnames,
        comment: form.comment.trim() || undefined,
      });
      toast.success(t("hosts.addSuccess", { ip: form.ip.trim() }));
      setForm(emptyAddEntryForm());
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
          <CardTitle>{t("hosts.newEntry")}</CardTitle>
          {!isAdministrator && (
            <Badge variant="outline" className="gap-1">
              <Lock className="size-3" />
              {t("common.administratorOnly")}
            </Badge>
          )}
        </div>
        <CardDescription>{t("hosts.newEntryDescription")}</CardDescription>
      </CardHeader>
      <CardContent className="flex flex-col gap-4">
        <div className="grid gap-4 sm:grid-cols-2">
          <div className="flex flex-col gap-1.5">
            <Label htmlFor="hostsIp">{t("hosts.ipLabel")}</Label>
            <Input
              id="hostsIp"
              placeholder={t("hosts.ipPlaceholder")}
              value={form.ip}
              onChange={(e) => update("ip", e.currentTarget.value)}
              disabled={!isAdministrator}
            />
          </div>
          <div className="flex flex-col gap-1.5">
            <Label htmlFor="hostsHostnames">{t("hosts.hostnamesLabel")}</Label>
            <Input
              id="hostsHostnames"
              placeholder={t("hosts.hostnamesPlaceholder")}
              value={form.hostnames}
              onChange={(e) => update("hostnames", e.currentTarget.value)}
              disabled={!isAdministrator}
            />
          </div>
        </div>
        <div className="flex flex-col gap-1.5">
          <Label htmlFor="hostsComment">{t("hosts.commentLabel")}</Label>
          <Input
            id="hostsComment"
            placeholder={t("hosts.commentPlaceholder")}
            value={form.comment}
            onChange={(e) => update("comment", e.currentTarget.value)}
            disabled={!isAdministrator}
          />
        </div>

        {error && <p className="text-sm text-destructive">{error}</p>}

        <div>
          <AdminRequiredTooltip locked={!isAdministrator}>
            <Button onClick={addEntry} disabled={!canAdd}>
              {!isAdministrator ? (
                <Lock />
              ) : adding ? (
                <Loader2 className="animate-spin" />
              ) : (
                <Plus />
              )}
              {adding ? t("hosts.adding") : t("hosts.addEntry")}
            </Button>
          </AdminRequiredTooltip>
        </div>
      </CardContent>
    </Card>
  );
}

function RawEditor({
  raw,
  t,
  isAdministrator,
  onSaved,
}: {
  raw: string;
  t: TFunction;
  isAdministrator: boolean;
  onSaved: () => void;
}) {
  const [value, setValue] = useState(raw);
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState<string | null>(null);

  async function save() {
    setSaving(true);
    setError(null);
    try {
      await invoke("set_hosts_raw", { content: value });
      toast.success(t("hosts.rawSaveSuccess"));
      onSaved();
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setSaving(false);
    }
  }

  return (
    <CollapsibleDetails label={t("hosts.rawEditorLabel")}>
      <div className="mt-3 flex flex-col gap-2">
        <p className="text-sm text-muted-foreground">
          {t("hosts.rawEditorDescription")}
        </p>
        <Textarea
          value={value}
          onChange={(e) => setValue(e.currentTarget.value)}
          disabled={!isAdministrator}
          className="min-h-64 font-mono text-xs"
          spellCheck={false}
        />
        {error && <p className="text-sm text-destructive">{error}</p>}
        <div>
          <AdminRequiredTooltip locked={!isAdministrator}>
            <Button
              variant="outline"
              onClick={save}
              disabled={!isAdministrator || saving}
            >
              {saving && <Loader2 className="animate-spin" />}
              {saving ? t("hosts.saving") : t("hosts.saveRaw")}
            </Button>
          </AdminRequiredTooltip>
        </div>
      </div>
    </CollapsibleDetails>
  );
}

export function HostsPage() {
  const { t } = useTranslation();
  const hosts = useHostsFile();
  const { isAdministrator } = useIsAdministrator();

  return (
    <div className="flex flex-col gap-6">
      <div>
        <h1 className="text-lg font-semibold">{t("hosts.title")}</h1>
        <p className="text-sm text-muted-foreground">{t("hosts.subtitle")}</p>
      </div>

      <div className="flex flex-col gap-3">
        <div className="flex items-center justify-between">
          <div className="flex items-center gap-2">
            {hosts.updatedAt && (
              <span className="text-xs text-muted-foreground">
                {t("common.updatedAgo", {
                  time: formatRelativeTime(t, hosts.updatedAt),
                })}
              </span>
            )}
          </div>
          <Button
            variant="ghost"
            size="icon-sm"
            onClick={hosts.refresh}
            disabled={hosts.status === "loading"}
          >
            <RefreshCw className={hosts.status === "loading" ? "animate-spin" : ""} />
          </Button>
        </div>

        {hosts.status === "loading" && hosts.entries.length === 0 && (
          <p className="flex items-center gap-2 text-sm text-muted-foreground">
            <Loader2 className="size-4 animate-spin" />
            {t("hosts.reading")}
          </p>
        )}
        {hosts.status === "error" && hosts.entries.length === 0 && (
          <p className="text-sm text-destructive">
            {t("hosts.couldntRead", { error: hosts.error })}
          </p>
        )}
        {hosts.status === "ready" && hosts.entries.length === 0 && (
          <p className="text-sm text-muted-foreground">{t("hosts.noneConfigured")}</p>
        )}
        {hosts.entries.length > 0 && (
          <EntryTable
            entries={hosts.entries}
            t={t}
            isAdministrator={isAdministrator}
            onChanged={hosts.refresh}
          />
        )}
      </div>

      <AddEntryForm t={t} isAdministrator={isAdministrator} onAdded={hosts.refresh} />

      {hosts.raw && (
        <Card>
          <CardContent>
            <RawEditor
              key={hosts.raw}
              raw={hosts.raw}
              t={t}
              isAdministrator={isAdministrator}
              onSaved={hosts.refresh}
            />
          </CardContent>
        </Card>
      )}
    </div>
  );
}
