import { useEffect, useState } from "react";
import { getVersion } from "@tauri-apps/api/app";

export function AppVersion() {
  const [version, setVersion] = useState<string | null>(null);

  useEffect(() => {
    getVersion()
      .then(setVersion)
      .catch(() => {});
  }, []);

  if (!version) return null;

  return (
    <span className="px-2 text-xs text-muted-foreground">v{version}</span>
  );
}
