import { useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { Loader2, ShieldAlert } from "lucide-react";
import { useTranslation } from "react-i18next";

import { Button } from "@/components/ui/button";
import { Card, CardContent } from "@/components/ui/card";
import { useIsAdministrator } from "@/lib/use-is-administrator";

export function AdminAlertCard() {
  const { t } = useTranslation();
  const { isAdministrator, status } = useIsAdministrator();
  const [relaunching, setRelaunching] = useState(false);
  const [error, setError] = useState<string | null>(null);

  if (status === "loading" || isAdministrator) return null;

  async function relaunch() {
    setRelaunching(true);
    setError(null);
    try {
      await invoke("relaunch_as_administrator");
      // On success the process exits from the Rust side; nothing left to do.
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
      setRelaunching(false);
    }
  }

  return (
    <Card className="border-amber-300 bg-amber-50 dark:border-amber-900 dark:bg-amber-950/30">
      <CardContent className="flex flex-col gap-3 sm:flex-row sm:items-center sm:justify-between">
        <div className="flex items-start gap-3">
          <ShieldAlert className="mt-0.5 size-5 shrink-0 text-amber-600 dark:text-amber-500" />
          <div className="flex flex-col gap-1">
            <p className="text-sm font-medium text-amber-900 dark:text-amber-200">
              {t("dashboard.adminAlert.title")}
            </p>
            <p className="text-sm text-amber-800/80 dark:text-amber-300/80">
              {t("dashboard.adminAlert.description")}
            </p>
            {error && (
              <p className="text-sm text-destructive">
                {t("dashboard.adminAlert.error", { error })}
              </p>
            )}
          </div>
        </div>
        <Button
          variant="outline"
          onClick={relaunch}
          disabled={relaunching}
          className="shrink-0"
        >
          {relaunching && <Loader2 className="animate-spin" />}
          {relaunching
            ? t("dashboard.adminAlert.relaunching")
            : t("dashboard.adminAlert.action")}
        </Button>
      </CardContent>
    </Card>
  );
}
