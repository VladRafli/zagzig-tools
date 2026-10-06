import { useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { Copy, Eye, Loader2, Lock, RefreshCw } from "lucide-react";
import { useTranslation } from "react-i18next";
import type { TFunction } from "i18next";
import { toast } from "sonner";

import { AdminRequiredTooltip } from "@/components/admin-required-tooltip";
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
import { useCachedInvoke } from "@/lib/use-cached-invoke";
import { formatRelativeTime } from "@/lib/relative-time";
import { useIsAdministrator } from "@/lib/use-is-administrator";

interface WifiProfile {
  name: string;
  authentication: string | null;
  encryption: string | null;
  connectionMode: string | null;
  autoSwitch: boolean;
}

interface WifiProfiles {
  available: boolean;
  profiles: WifiProfile[];
}

const GRID = "grid-cols-[1.4fr_1fr_6rem_6rem_auto]";

// Shows one profile's saved password. The key is requested only when the
// user confirms, lives only in this component's state, and is gone as soon
// as the dialog closes — it is never cached or written anywhere.
function RevealDialog({
  profile,
  t,
  onClose,
}: {
  profile: WifiProfile | null;
  t: TFunction;
  onClose: () => void;
}) {
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [revealed, setRevealed] = useState<{ key: string | null } | null>(null);

  function close() {
    if (busy) return;
    setRevealed(null);
    setError(null);
    onClose();
  }

  async function reveal() {
    if (!profile) return;
    setBusy(true);
    setError(null);
    try {
      const key = await invoke<string | null>("reveal_wifi_key", { name: profile.name });
      setRevealed({ key });
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setBusy(false);
    }
  }

  async function copy(key: string) {
    try {
      await navigator.clipboard.writeText(key);
      toast.success(t("wifi.copied"));
    } catch {
      toast.error(t("wifi.copyFailed"));
    }
  }

  return (
    <Dialog open={profile !== null} onOpenChange={(open) => !open && close()}>
      <DialogContent>
        <DialogHeader>
          <DialogTitle>{t("wifi.reveal.title", { name: profile?.name ?? "" })}</DialogTitle>
          <DialogDescription>
            {revealed ? t("wifi.reveal.shownDescription") : t("wifi.reveal.description")}
          </DialogDescription>
        </DialogHeader>
        {revealed &&
          (revealed.key ? (
            <div className="flex items-center gap-2 rounded-lg border bg-muted/30 p-3">
              <code className="min-w-0 flex-1 break-all text-sm">{revealed.key}</code>
              <Button variant="outline" size="sm" onClick={() => copy(revealed.key as string)}>
                <Copy />
                {t("wifi.copy")}
              </Button>
            </div>
          ) : (
            <p className="text-sm text-muted-foreground">{t("wifi.reveal.noKey")}</p>
          ))}
        {error && <p className="text-sm text-destructive">{error}</p>}
        <DialogFooter>
          <Button variant="outline" onClick={close} disabled={busy}>
            {revealed ? t("wifi.close") : t("wifi.cancel")}
          </Button>
          {!revealed && (
            <Button onClick={reveal} disabled={busy}>
              {busy && <Loader2 className="animate-spin" />}
              {busy ? t("wifi.reveal.working") : t("wifi.reveal.confirm")}
            </Button>
          )}
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}

export function WifiPage() {
  const { t } = useTranslation();
  const { data, status, error, updatedAt, refresh } = useCachedInvoke<WifiProfiles>(
    "zagzig:wifi",
    "get_wifi_profiles",
    30 * 1000,
  );
  const { isAdministrator } = useIsAdministrator();
  const [target, setTarget] = useState<WifiProfile | null>(null);

  const profiles = data?.profiles ?? [];

  return (
    <div className="flex flex-col gap-6">
      <div>
        <h1 className="text-lg font-semibold">{t("wifi.title")}</h1>
        <p className="text-sm text-muted-foreground">{t("wifi.subtitle")}</p>
      </div>

      <div className="flex flex-col gap-3">
        <div className="flex items-center justify-end gap-2">
          {updatedAt && (
            <span className="text-xs text-muted-foreground">
              {t("common.updatedAgo", { time: formatRelativeTime(t, updatedAt) })}
            </span>
          )}
          <Button variant="ghost" size="icon-sm" onClick={refresh} disabled={status === "loading"}>
            <RefreshCw className={status === "loading" ? "animate-spin" : ""} />
          </Button>
        </div>

        {status === "loading" && !data && (
          <p className="flex items-center gap-2 text-sm text-muted-foreground">
            <Loader2 className="size-4 animate-spin" />
            {t("wifi.reading")}
          </p>
        )}
        {status === "error" && !data && (
          <p className="text-sm text-destructive">{t("wifi.couldntRead", { error })}</p>
        )}
        {data && !data.available && (
          <p className="text-sm text-muted-foreground">{t("wifi.unavailable")}</p>
        )}
        {data?.available && profiles.length === 0 && (
          <p className="text-sm text-muted-foreground">{t("wifi.none")}</p>
        )}
        {profiles.length > 0 && (
          <div className="overflow-x-auto rounded-lg border">
            <div className="min-w-[40rem]">
              <div
                className={`grid ${GRID} gap-2 border-b bg-muted/50 px-3 py-2 text-xs font-medium text-muted-foreground`}
              >
                <span>{t("wifi.columns.name")}</span>
                <span>{t("wifi.columns.security")}</span>
                <span>{t("wifi.columns.connect")}</span>
                <span>{t("wifi.columns.autoSwitch")}</span>
                <span />
              </div>
              {profiles.map((p) => (
                <div
                  key={p.name}
                  className={`grid ${GRID} items-center gap-2 border-b px-3 py-2 text-sm last:border-b-0`}
                >
                  <span className="break-all font-medium">{p.name}</span>
                  <span className="text-xs text-muted-foreground">
                    {[p.authentication, p.encryption].filter(Boolean).join(" / ") || t("common.none")}
                  </span>
                  <Badge variant="outline" className="w-fit">
                    {p.connectionMode === "auto" ? t("wifi.auto") : t("wifi.manual")}
                  </Badge>
                  <span className="text-xs text-muted-foreground">
                    {p.autoSwitch ? t("common.yes") : t("common.no")}
                  </span>
                  <div className="flex justify-end">
                    <AdminRequiredTooltip locked={!isAdministrator}>
                      <Button
                        variant="outline"
                        size="sm"
                        disabled={!isAdministrator || p.authentication === "open"}
                        onClick={() => setTarget(p)}
                      >
                        {isAdministrator ? <Eye /> : <Lock />}
                        {t("wifi.showPassword")}
                      </Button>
                    </AdminRequiredTooltip>
                  </div>
                </div>
              ))}
            </div>
          </div>
        )}
      </div>

      <RevealDialog profile={target} t={t} onClose={() => setTarget(null)} />
    </div>
  );
}
