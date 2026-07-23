import { useState } from "react";
import { relaunch } from "@tauri-apps/plugin-process";
import { Loader2, RefreshCw, SparkleIcon } from "lucide-react";
import { useTranslation } from "react-i18next";
import { toast } from "sonner";

import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
  DialogTrigger,
} from "@/components/ui/dialog";
import { useUpdateChecker } from "@/lib/use-update-checker";

export function UpdateButton() {
  const { t } = useTranslation();
  const { status, update, checkForUpdates } = useUpdateChecker();
  const [open, setOpen] = useState(false);
  const [installing, setInstalling] = useState(false);
  const [percent, setPercent] = useState<number | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [manualChecking, setManualChecking] = useState(false);

  function handleOpenChange(next: boolean) {
    if (installing) return;
    setOpen(next);
    if (next) setError(null);
  }

  async function installUpdate() {
    if (!update) return;
    setInstalling(true);
    setError(null);
    setPercent(null);

    let total = 0;
    let downloaded = 0;
    try {
      await update.downloadAndInstall((event) => {
        if (event.event === "Started") {
          total = event.data.contentLength ?? 0;
        } else if (event.event === "Progress") {
          downloaded += event.data.chunkLength;
          setPercent(total > 0 ? Math.round((downloaded / total) * 100) : null);
        }
      });
      await relaunch();
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
      setInstalling(false);
    }
  }

  // Manual checks always give feedback (a toast either way) — background
  // checks (startup + hourly) stay silent unless they actually find
  // something, which shows up as the "available" button below instead.
  async function handleManualCheck() {
    setManualChecking(true);
    try {
      const result = await checkForUpdates();
      if (result === "idle") {
        toast.success(t("app.update.upToDate"));
      } else if (result === "error") {
        toast.error(t("app.update.checkError"));
      }
    } finally {
      setManualChecking(false);
    }
  }

  if (status === "available" && update) {
    return (
      <Dialog open={open} onOpenChange={handleOpenChange}>
        <DialogTrigger
          render={<Button variant="outline" size="sm" className="w-full justify-start gap-2" />}
        >
          <SparkleIcon className="text-primary" />
          {t("app.update.available")}
          <Badge variant="default" className="ml-auto">
            {update.version}
          </Badge>
        </DialogTrigger>
        <DialogContent>
          <DialogHeader>
            <DialogTitle>
              {t("app.update.dialogTitle", { version: update.version })}
            </DialogTitle>
            <DialogDescription>
              {t("app.update.dialogDescription", {
                current: update.currentVersion,
                next: update.version,
              })}
            </DialogDescription>
          </DialogHeader>
          <div className="max-h-48 overflow-y-auto rounded-lg border bg-muted/30 p-3 text-sm whitespace-pre-wrap">
            {update.body || t("app.update.noReleaseNotes")}
          </div>
          {error && <p className="text-sm text-destructive">{t("app.update.error", { error })}</p>}
          <DialogFooter>
            <Button variant="outline" onClick={() => setOpen(false)} disabled={installing}>
              {t("app.update.cancel")}
            </Button>
            <Button onClick={installUpdate} disabled={installing}>
              {installing && <Loader2 className="animate-spin" />}
              {installing
                ? percent !== null
                  ? t("app.update.installingProgress", { percent })
                  : t("app.update.installing")
                : t("app.update.install")}
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    );
  }

  const busy = status === "checking" || manualChecking;
  return (
    <Button
      variant="ghost"
      size="sm"
      className="w-full justify-start gap-2 text-muted-foreground"
      onClick={handleManualCheck}
      disabled={busy}
    >
      {busy ? <Loader2 className="animate-spin" /> : <RefreshCw />}
      {busy ? t("app.update.checking") : t("app.update.checkForUpdates")}
    </Button>
  );
}
