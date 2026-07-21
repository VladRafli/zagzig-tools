import { useCallback, useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";

const OVERRIDE_KEY = "zagzig:signtool-path-override";

interface SigntoolStatus {
  found: boolean;
  path: string | null;
}

function readOverride(): string | null {
  try {
    return localStorage.getItem(OVERRIDE_KEY);
  } catch {
    return null;
  }
}

function writeOverride(path: string | null) {
  try {
    if (path) localStorage.setItem(OVERRIDE_KEY, path);
    else localStorage.removeItem(OVERRIDE_KEY);
  } catch {
    // ignore — best-effort
  }
}

// Locates signtool.exe: first an auto-detect sweep of the Windows SDK
// install locations (see `find_signtool` in Rust), falling back to a
// manually browsed-to path the user picked before, persisted across
// restarts since re-detecting is cheap but re-asking the user isn't.
export function useSigntool() {
  const [status, setStatus] = useState<"loading" | "ready">("loading");
  const [found, setFound] = useState(false);
  const [path, setPath] = useState<string | null>(null);
  const [isOverride, setIsOverride] = useState(false);

  const detect = useCallback((overridePath: string | null) => {
    setStatus("loading");
    invoke<SigntoolStatus>("find_signtool", {
      customPath: overridePath ?? undefined,
    })
      .then((result) => {
        setFound(result.found);
        setPath(result.path);
        setIsOverride(overridePath !== null);
        setStatus("ready");
      })
      .catch(() => {
        setFound(false);
        setPath(null);
        setIsOverride(false);
        setStatus("ready");
      });
  }, []);

  useEffect(() => {
    detect(readOverride());
  }, [detect]);

  const locate = useCallback(async () => {
    const selected = await open({
      title: "Locate signtool.exe",
      filters: [{ name: "signtool.exe", extensions: ["exe"] }],
      multiple: false,
    });
    if (typeof selected === "string") {
      writeOverride(selected);
      detect(selected);
    }
  }, [detect]);

  const clearOverride = useCallback(() => {
    writeOverride(null);
    detect(null);
  }, [detect]);

  return { status, found, path, isOverride, locate, clearOverride };
}
