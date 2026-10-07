import { useCallback, useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { DatabaseBackup, GitCompare, Loader2, Lock, RotateCcw, Trash2 } from "lucide-react";
import { useTranslation } from "react-i18next";
import { toast } from "sonner";

import { AdminRequiredTooltip } from "@/components/admin-required-tooltip";
import { CollapsibleDetails } from "@/components/collapsible-details";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";

interface HostsBackup {
  file: string;
  time: number;
  reason: string;
  size: number;
  lines: number;
}

function errorMessage(err: unknown) {
  return err instanceof Error ? err.message : String(err);
}

// Lines that differ between two versions of the file, ignoring blanks and
// order: what the backup has that the current file doesn't, and vice versa.
export function diffLines(backup: string, current: string) {
  const clean = (text: string) => text.split(/\r?\n/).map((l) => l.trim()).filter(Boolean);
  const count = (lines: string[]) => {
    const map = new Map<string, number>();
    lines.forEach((l) => map.set(l, (map.get(l) ?? 0) + 1));
    return map;
  };
  const b = clean(backup);
  const c = clean(current);
  const inBackup = count(b);
  const inCurrent = count(c);
  const onlyBackup = b.filter((line) => {
    const left = inCurrent.get(line) ?? 0;
    if (left > 0) {
      inCurrent.set(line, left - 1);
      return false;
    }
    return true;
  });
  const onlyCurrent = c.filter((line) => {
    const left = inBackup.get(line) ?? 0;
    if (left > 0) {
      inBackup.set(line, left - 1);
      return false;
    }
    return true;
  });
  return { onlyBackup, onlyCurrent };
}

export function HostsBackups({
  currentRaw,
  isAdministrator,
  onRestored,
}: {
  currentRaw: string;
  isAdministrator: boolean;
  onRestored: () => void;
}) {
  const { t } = useTranslation();
  const [backups, setBackups] = useState<HostsBackup[]>([]);
  const [busy, setBusy] = useState(false);
  const [comparing, setComparing] = useState<{ backup: HostsBackup; content: string } | null>(null);
  const [error, setError] = useState<string | null>(null);

  const load = useCallback(() => {
    invoke<HostsBackup[]>("list_hosts_backups")
      .then(setBackups)
      .catch(() => setBackups([]));
  }, []);

  // Reload when the file changes, since every change makes a backup.
  useEffect(() => {
    load();
  }, [load, currentRaw]);

  async function backUpNow() {
    setBusy(true);
    try {
      const created = await invoke<boolean>("create_hosts_backup");
      toast.success(t(created ? "hosts.backups.created" : "hosts.backups.upToDate"));
      load();
    } catch (err) {
      toast.error(errorMessage(err));
    } finally {
      setBusy(false);
    }
  }

  async function compare(backup: HostsBackup) {
    try {
      const content = await invoke<string>("read_hosts_backup", { file: backup.file });
      setError(null);
      setComparing({ backup, content });
    } catch (err) {
      toast.error(errorMessage(err));
    }
  }

  async function remove(backup: HostsBackup) {
    try {
      await invoke("delete_hosts_backup", { file: backup.file });
      load();
    } catch (err) {
      toast.error(errorMessage(err));
    }
  }

  async function restore() {
    if (!comparing) return;
    setBusy(true);
    setError(null);
    try {
      await invoke("restore_hosts_backup", { file: comparing.backup.file });
      toast.success(t("hosts.backups.restored"));
      setComparing(null);
      onRestored();
      load();
    } catch (err) {
      setError(errorMessage(err));
    } finally {
      setBusy(false);
    }
  }

  const diff = comparing ? diffLines(comparing.content, currentRaw) : null;
  const identical = diff ? diff.onlyBackup.length === 0 && diff.onlyCurrent.length === 0 : false;

  return (
    <CollapsibleDetails label={t("hosts.backups.label")}>
      <div className="mt-3 flex flex-col gap-3">
        <p className="text-sm text-muted-foreground">{t("hosts.backups.description")}</p>
        <div>
          <Button variant="outline" size="sm" onClick={backUpNow} disabled={busy}>
            {busy ? <Loader2 className="animate-spin" /> : <DatabaseBackup />}
            {t("hosts.backups.backUpNow")}
          </Button>
        </div>

        {backups.length === 0 ? (
          <p className="text-sm text-muted-foreground">{t("hosts.backups.none")}</p>
        ) : (
          <div className="rounded-lg border">
            {backups.map((b) => (
              <div key={b.file} className="flex flex-wrap items-center gap-2 border-b px-3 py-2 text-sm last:border-b-0">
                <div className="min-w-0 flex-1">
                  <span>{new Date(b.time * 1000).toLocaleString()}</span>
                  <Badge variant="outline" className="ml-2">
                    {t(`hosts.backups.reasons.${b.reason}`)}
                  </Badge>
                  <span className="ml-2 text-xs text-muted-foreground">
                    {t("hosts.backups.lines", { count: b.lines })}
                  </span>
                </div>
                <Button variant="outline" size="sm" onClick={() => compare(b)}>
                  <GitCompare />
                  {t("hosts.backups.compare")}
                </Button>
                <Button
                  variant="ghost"
                  size="icon-sm"
                  aria-label={t("hosts.backups.delete")}
                  title={t("hosts.backups.delete")}
                  onClick={() => remove(b)}
                >
                  <Trash2 />
                </Button>
              </div>
            ))}
          </div>
        )}
      </div>

      <Dialog open={comparing !== null} onOpenChange={(open) => !open && !busy && setComparing(null)}>
        <DialogContent className="sm:max-w-2xl">
          <DialogHeader>
            <DialogTitle>{t("hosts.backups.compareTitle")}</DialogTitle>
            <DialogDescription>
              {comparing && new Date(comparing.backup.time * 1000).toLocaleString()}
            </DialogDescription>
          </DialogHeader>
          {diff && (
            <div className="flex max-h-80 flex-col gap-3 overflow-y-auto text-sm">
              {identical && <p className="text-muted-foreground">{t("hosts.backups.identical")}</p>}
              {diff.onlyBackup.length > 0 && (
                <div>
                  <p className="mb-1 text-xs font-medium text-muted-foreground">
                    {t("hosts.backups.restoreWouldAdd", { count: diff.onlyBackup.length })}
                  </p>
                  <pre className="overflow-x-auto rounded-lg border border-green-600/30 bg-green-600/5 p-2 text-xs">
                    {diff.onlyBackup.map((l) => `+ ${l}`).join("\n")}
                  </pre>
                </div>
              )}
              {diff.onlyCurrent.length > 0 && (
                <div>
                  <p className="mb-1 text-xs font-medium text-muted-foreground">
                    {t("hosts.backups.restoreWouldRemove", { count: diff.onlyCurrent.length })}
                  </p>
                  <pre className="overflow-x-auto rounded-lg border border-destructive/30 bg-destructive/5 p-2 text-xs">
                    {diff.onlyCurrent.map((l) => `- ${l}`).join("\n")}
                  </pre>
                </div>
              )}
            </div>
          )}
          <p className="text-xs text-muted-foreground">{t("hosts.backups.restoreNote")}</p>
          {error && <p className="text-sm text-destructive">{error}</p>}
          <DialogFooter>
            <Button variant="outline" onClick={() => setComparing(null)} disabled={busy}>
              {t("hosts.backups.close")}
            </Button>
            <AdminRequiredTooltip locked={!isAdministrator}>
              <Button onClick={restore} disabled={!isAdministrator || busy || identical}>
                {!isAdministrator ? <Lock /> : busy ? <Loader2 className="animate-spin" /> : <RotateCcw />}
                {t("hosts.backups.restore")}
              </Button>
            </AdminRequiredTooltip>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </CollapsibleDetails>
  );
}
