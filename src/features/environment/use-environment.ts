import { useCachedInvoke } from "@/lib/use-cached-invoke";

const CACHE_KEY = "zagzig:environment";

// Cheap to read and edited from elsewhere (installers, setx, ...), so keep
// the cached copy short-lived.
const TTL_MS = 10 * 1000;

export type EnvScope = "user" | "system";
export type EnvKind = "string" | "expand";

export interface EnvVar {
  name: string;
  value: string;
  kind: EnvKind;
}

interface EnvSnapshot {
  user: EnvVar[];
  system: EnvVar[];
}

export interface EnvChange {
  id: string;
  time: number;
  scope: EnvScope;
  name: string;
  action: "set" | "delete" | "undo";
  previous: { value: string; kind: EnvKind } | null;
}

// Mirrors the backend's list of machine variables Windows depends on; the
// backend enforces it, this just avoids offering a button that will fail.
const PROTECTED_SYSTEM = new Set([
  "path",
  "pathext",
  "comspec",
  "systemroot",
  "windir",
  "systemdrive",
  "os",
  "temp",
  "tmp",
  "psmodulepath",
  "driverdata",
  "processor_architecture",
  "numberof_processors",
]);

export function isProtected(scope: EnvScope, name: string): boolean {
  return scope === "system" && PROTECTED_SYSTEM.has(name.toLowerCase());
}

/** PATH-like variables hold folders, so their entries can be checked. */
export function isPathList(name: string): boolean {
  const n = name.toLowerCase();
  return n !== "pathext" && (n === "path" || n.endsWith("path"));
}

export function hasVariableReference(value: string): boolean {
  return /%[^%\r\n]+%/.test(value);
}

export function useEnvironment() {
  const { data, status, error, updatedAt, refresh } = useCachedInvoke<EnvSnapshot>(
    CACHE_KEY,
    "get_env_variables",
    TTL_MS,
  );

  return {
    user: data?.user ?? [],
    system: data?.system ?? [],
    status,
    error,
    updatedAt,
    refresh,
  };
}
