import { useEffect, useMemo, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { AlertTriangle, ArrowDown, ArrowUp, Check, Plus, X } from "lucide-react";
import type { TFunction } from "i18next";

import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";

interface PathCheck {
  expanded: string;
  exists: boolean | null;
}

// Case-insensitive, ignoring a trailing slash — how Windows compares folders.
function normalize(entry: string): string {
  return entry.trim().replace(/[\\/]+$/, "").toLowerCase();
}

export function splitList(value: string): string[] {
  return value.split(";").filter((entry) => entry.trim() !== "");
}

export function joinList(entries: string[]): string {
  return entries.map((e) => e.trim()).filter(Boolean).join(";");
}

/**
 * Edits a `;`-separated variable one entry per row: reorder, remove,
 * and — for PATH-like variables — see which folders exist and which entries
 * are duplicates.
 */
export function ListEditor({
  entries,
  onChange,
  checkFolders,
  disabled,
  t,
}: {
  entries: string[];
  onChange: (entries: string[]) => void;
  checkFolders: boolean;
  disabled: boolean;
  t: TFunction;
}) {
  const [checks, setChecks] = useState<Record<string, PathCheck>>({});

  // Look up which folders exist, a moment after the user stops typing.
  useEffect(() => {
    if (!checkFolders) return;
    const unique = [...new Set(entries.map((e) => e.trim()).filter(Boolean))];
    const handle = setTimeout(() => {
      invoke<PathCheck[]>("check_path_entries", { entries: unique })
        .then((results) => {
          const next: Record<string, PathCheck> = {};
          unique.forEach((entry, i) => (next[entry] = results[i]));
          setChecks(next);
        })
        .catch(() => setChecks({}));
    }, 300);
    return () => clearTimeout(handle);
  }, [entries, checkFolders]);

  const duplicates = useMemo(() => {
    const seen = new Set<string>();
    const dup = new Set<number>();
    entries.forEach((entry, i) => {
      const key = normalize(entry);
      if (!key) return;
      if (seen.has(key)) dup.add(i);
      seen.add(key);
    });
    return dup;
  }, [entries]);

  const missing = useMemo(
    () => entries.filter((e) => checks[e.trim()]?.exists === false).length,
    [entries, checks],
  );

  function update(index: number, value: string) {
    onChange(entries.map((e, i) => (i === index ? value : e)));
  }

  function move(index: number, delta: number) {
    const target = index + delta;
    if (target < 0 || target >= entries.length) return;
    const next = [...entries];
    [next[index], next[target]] = [next[target], next[index]];
    onChange(next);
  }

  return (
    <div className="flex flex-col gap-2">
      <div className="flex max-h-72 flex-col gap-1.5 overflow-y-auto pr-1">
        {entries.length === 0 && (
          <p className="text-sm text-muted-foreground">{t("environment.list.empty")}</p>
        )}
        {entries.map((entry, index) => {
          const check = checks[entry.trim()];
          return (
            <div key={index} className="flex items-center gap-1.5">
              <Input
                className="font-mono text-xs"
                value={entry}
                disabled={disabled}
                onChange={(e) => update(index, e.currentTarget.value)}
                aria-label={t("environment.list.entry", { number: index + 1 })}
              />
              <span className="flex w-24 shrink-0 items-center gap-1 text-xs">
                {duplicates.has(index) && (
                  <span className="flex items-center gap-0.5 text-amber-600 dark:text-amber-400">
                    <AlertTriangle className="size-3.5" />
                    {t("environment.list.duplicate")}
                  </span>
                )}
                {!duplicates.has(index) && checkFolders && check?.exists === true && (
                  <span className="flex items-center gap-0.5 text-muted-foreground">
                    <Check className="size-3.5" />
                    {t("environment.list.exists")}
                  </span>
                )}
                {!duplicates.has(index) && checkFolders && check?.exists === false && (
                  <span className="flex items-center gap-0.5 text-destructive">
                    <AlertTriangle className="size-3.5" />
                    {t("environment.list.missing")}
                  </span>
                )}
              </span>
              <Button
                variant="ghost"
                size="icon-sm"
                aria-label={t("environment.list.moveUp")}
                disabled={disabled || index === 0}
                onClick={() => move(index, -1)}
              >
                <ArrowUp />
              </Button>
              <Button
                variant="ghost"
                size="icon-sm"
                aria-label={t("environment.list.moveDown")}
                disabled={disabled || index === entries.length - 1}
                onClick={() => move(index, 1)}
              >
                <ArrowDown />
              </Button>
              <Button
                variant="ghost"
                size="icon-sm"
                aria-label={t("environment.list.remove")}
                disabled={disabled}
                onClick={() => onChange(entries.filter((_, i) => i !== index))}
              >
                <X />
              </Button>
            </div>
          );
        })}
      </div>
      <div className="flex flex-wrap items-center gap-2">
        <Button variant="outline" size="sm" disabled={disabled} onClick={() => onChange([...entries, ""])}>
          <Plus />
          {t("environment.list.add")}
        </Button>
        {duplicates.size > 0 && (
          <Button
            variant="outline"
            size="sm"
            disabled={disabled}
            onClick={() => onChange(entries.filter((_, i) => !duplicates.has(i)))}
          >
            {t("environment.list.removeDuplicates", { count: duplicates.size })}
          </Button>
        )}
        {checkFolders && missing > 0 && (
          <Button
            variant="outline"
            size="sm"
            disabled={disabled}
            onClick={() => onChange(entries.filter((e) => checks[e.trim()]?.exists !== false))}
          >
            {t("environment.list.removeMissing", { count: missing })}
          </Button>
        )}
      </div>
    </div>
  );
}
