import { useCallback, useEffect, useRef, useState } from "react";
import { check, type Update } from "@tauri-apps/plugin-updater";

export type UpdateCheckStatus = "checking" | "idle" | "available" | "error";

// How often to silently re-check in the background. GitHub's latest.json
// endpoint is a release-asset download (served off their CDN), not a REST
// API call, so this doesn't burn API rate limit — the interval is purely
// about not checking more often than is useful.
const CHECK_INTERVAL_MS = 60 * 60 * 1000;

// Checks GitHub's latest release (via the `latest.json` the release
// workflow publishes alongside the installers) on startup, again every hour
// in the background, and on demand via checkForUpdates() (used by the
// manual "Check for updates" button). Background checks only fire while
// idle/errored — never while an update is already known about or being
// installed, so they can't race with an in-progress install.
export function useUpdateChecker() {
  const [status, setStatus] = useState<UpdateCheckStatus>("checking");
  const [update, setUpdate] = useState<Update | null>(null);
  const [error, setError] = useState<string | null>(null);
  const updateRef = useRef<Update | null>(null);
  const statusRef = useRef<UpdateCheckStatus>(status);

  useEffect(() => {
    statusRef.current = status;
  }, [status]);

  const checkForUpdates = useCallback(async (): Promise<UpdateCheckStatus> => {
    setStatus("checking");
    setError(null);
    try {
      const result = await check();
      await updateRef.current?.close();
      updateRef.current = result;
      setUpdate(result);
      const nextStatus: UpdateCheckStatus = result ? "available" : "idle";
      setStatus(nextStatus);
      return nextStatus;
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
      setStatus("error");
      return "error";
    }
  }, []);

  useEffect(() => {
    checkForUpdates();
    return () => {
      updateRef.current?.close();
    };
  }, [checkForUpdates]);

  useEffect(() => {
    const interval = setInterval(() => {
      if (statusRef.current === "idle" || statusRef.current === "error") {
        checkForUpdates();
      }
    }, CHECK_INTERVAL_MS);
    return () => clearInterval(interval);
  }, [checkForUpdates]);

  return { status, update, error, checkForUpdates };
}
