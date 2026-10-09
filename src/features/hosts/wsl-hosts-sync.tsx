import { useCallback, useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { Loader2, RefreshCw, Trash2, Upload } from "lucide-react";
import { useTranslation } from "react-i18next";
import { toast } from "sonner";

import { CollapsibleDetails } from "@/components/collapsible-details";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Switch } from "@/components/ui/switch";
import type { WslDistro, WslStatus } from "@/features/wsl/use-wsl";

interface SyncStatus {
  syncedEntries: number | null;
  generateHosts: boolean;
  autoSync: boolean;
  bootConflict: string | null;
}

interface SyncResult {
  entries: number;
  skipped: number;
}

function errorMessage(err: unknown) {
  return err instanceof Error ? err.message : String(err);
}

// Docker Desktop's own distributions aren't places to put hosts entries.
const isInternal = (d: WslDistro) => d.name.startsWith("docker-desktop");

function DistroRow({
  distro,
  windowsHosts,
  onChanged,
}: {
  distro: WslDistro;
  windowsHosts: number;
  onChanged: () => void;
}) {
  const { t } = useTranslation();
  const [status, setStatus] = useState<SyncStatus | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);

  const load = useCallback(async () => {
    try {
      setStatus(await invoke<SyncStatus>("wsl_hosts_status", { name: distro.name }));
      setError(null);
    } catch (err) {
      setError(errorMessage(err));
    }
  }, [distro.name]);

  // Reading a stopped distribution would start it, so only running ones are
  // read without being asked.
  useEffect(() => {
    if (distro.running) load();
  }, [distro.running, load]);

  async function run(action: () => Promise<void>) {
    setBusy(true);
    try {
      await action();
      await load();
      onChanged();
    } catch (err) {
      toast.error(errorMessage(err));
    } finally {
      setBusy(false);
    }
  }

  const sync = () =>
    run(async () => {
      const result = await invoke<SyncResult>("wsl_hosts_sync", { name: distro.name });
      toast.success(t("hosts.wslSync.synced", { count: result.entries, name: distro.name }));
      if (result.skipped > 0) toast.message(t("hosts.wslSync.skipped", { count: result.skipped }));
    });

  const remove = () =>
    run(async () => {
      await invoke("wsl_hosts_remove", { name: distro.name });
      toast.success(t("hosts.wslSync.removed", { name: distro.name }));
    });

  const toggleAuto = (enabled: boolean) =>
    run(async () => {
      await invoke("wsl_hosts_autosync", { name: distro.name, enabled });
      toast.success(t(enabled ? "hosts.wslSync.autoOn" : "hosts.wslSync.autoOff", { name: distro.name }));
    });

  const synced = status?.syncedEntries != null;
  const wontLast = status && status.generateHosts && !status.autoSync;

  return (
    <div className="flex flex-col gap-2 border-b px-3 py-3 text-sm last:border-b-0">
      <div className="flex flex-wrap items-center gap-2">
        <span className="font-medium">{distro.name}</span>
        {distro.isDefault && <Badge variant="secondary">{t("wsl.default")}</Badge>}
        <Badge variant="outline">{t(distro.running ? "hosts.wslSync.running" : "hosts.wslSync.stopped")}</Badge>
        <span className="text-xs text-muted-foreground">
          {status
            ? synced
              ? t("hosts.wslSync.hasBlock", { count: status.syncedEntries ?? 0 })
              : t("hosts.wslSync.noBlock")
            : distro.running
              ? t("hosts.wslSync.reading")
              : t("hosts.wslSync.startsOnSync")}
        </span>
        <div className="ml-auto flex items-center gap-2">
          <Button size="sm" variant="outline" onClick={sync} disabled={busy || windowsHosts === 0}>
            {busy ? <Loader2 className="animate-spin" /> : <Upload />}
            {t("hosts.wslSync.syncNow")}
          </Button>
          {synced && (
            <Button
              size="icon-sm"
              variant="ghost"
              onClick={remove}
              disabled={busy}
              aria-label={t("hosts.wslSync.remove")}
              title={t("hosts.wslSync.remove")}
            >
              <Trash2 />
            </Button>
          )}
        </div>
      </div>

      {status && (
        <div className="flex items-start gap-3">
          <Switch
            id={`auto-${distro.name}`}
            checked={status.autoSync}
            disabled={busy || (status.bootConflict !== null && !status.autoSync)}
            onCheckedChange={toggleAuto}
          />
          <label htmlFor={`auto-${distro.name}`} className="flex flex-col gap-0.5">
            <span>{t("hosts.wslSync.keepSynced")}</span>
            <span className="text-xs text-muted-foreground">
              {status.bootConflict
                ? t("hosts.wslSync.conflict", { command: status.bootConflict })
                : status.autoSync
                  ? t("hosts.wslSync.keepSyncedOn")
                  : !status.generateHosts
                    ? t("hosts.wslSync.keepSyncedNotNeeded")
                    : t("hosts.wslSync.keepSyncedHint")}
            </span>
          </label>
        </div>
      )}
      {wontLast && synced && (
        <p className="text-xs text-amber-600 dark:text-amber-400">{t("hosts.wslSync.wontLast")}</p>
      )}
      {error && <p className="text-xs text-destructive">{error}</p>}
    </div>
  );
}

export function WslHostsSync({ activeEntries, onChanged }: { activeEntries: number; onChanged?: () => void }) {
  const { t } = useTranslation();
  const [distros, setDistros] = useState<WslDistro[] | null>(null);
  const [problem, setProblem] = useState<string | null>(null);
  const [loading, setLoading] = useState(false);
  const [version, setVersion] = useState(0);

  const load = useCallback(async () => {
    setLoading(true);
    try {
      const status = await invoke<WslStatus>("get_wsl_status");
      if (!status.installed) setProblem(t("hosts.wslSync.notInstalled"));
      else if (status.unresponsive) setProblem(t("hosts.wslSync.unresponsive"));
      else setProblem(null);
      setDistros(status.distros.filter((d) => !isInternal(d)));
    } catch (err) {
      setProblem(errorMessage(err));
      setDistros([]);
    } finally {
      setLoading(false);
    }
  }, [t]);

  useEffect(() => {
    load();
  }, [load]);

  return (
    <CollapsibleDetails label={t("hosts.wslSync.label")}>
      <div className="mt-3 flex flex-col gap-3">
        <p className="text-sm text-muted-foreground">{t("hosts.wslSync.description")}</p>
        <p className="text-xs text-muted-foreground">{t("hosts.wslSync.notes")}</p>
        <div>
          <Button variant="ghost" size="sm" onClick={() => { setVersion((v) => v + 1); load(); }} disabled={loading}>
            {loading ? <Loader2 className="animate-spin" /> : <RefreshCw />}
            {t("hosts.wslSync.refresh")}
          </Button>
        </div>
        {problem && <p className="text-sm text-destructive">{problem}</p>}
        {distros && distros.length === 0 && !problem && (
          <p className="text-sm text-muted-foreground">{t("hosts.wslSync.noDistros")}</p>
        )}
        {distros && distros.length > 0 && (
          <div className="rounded-lg border">
            {distros.map((d) => (
              <DistroRow
                key={`${d.name}-${version}`}
                distro={d}
                windowsHosts={activeEntries}
                onChanged={() => onChanged?.()}
              />
            ))}
          </div>
        )}
      </div>
    </CollapsibleDetails>
  );
}
