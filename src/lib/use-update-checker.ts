import { useCallback, useEffect, useRef, useState } from "react";
import { check, type Update } from "@tauri-apps/plugin-updater";

export type UpdateCheckStatus = "checking" | "idle" | "available" | "error";

// Checks GitHub's latest release (via the `latest.json` the release
// workflow publishes alongside the installers) once on startup. Kept as a
// hook rather than a Tauri command since the updater plugin's JS API
// already does the version comparison against the running app itself.
export function useUpdateChecker() {
  const [status, setStatus] = useState<UpdateCheckStatus>("checking");
  const [update, setUpdate] = useState<Update | null>(null);
  const [error, setError] = useState<string | null>(null);
  const updateRef = useRef<Update | null>(null);

  const checkForUpdates = useCallback(async () => {
    setStatus("checking");
    setError(null);
    try {
      const result = await check();
      await updateRef.current?.close();
      updateRef.current = result;
      setUpdate(result);
      setStatus(result ? "available" : "idle");
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
      setStatus("error");
    }
  }, []);

  useEffect(() => {
    checkForUpdates();
    return () => {
      updateRef.current?.close();
    };
  }, [checkForUpdates]);

  return { status, update, error, checkForUpdates };
}
