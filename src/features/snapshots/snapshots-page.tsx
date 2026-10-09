import { useCallback, useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { Camera, GitCompareArrows, Loader2, Pencil, RefreshCw, Trash2 } from "lucide-react";
import { useTranslation } from "react-i18next";
import type { TFunction } from "i18next";
import { toast } from "sonner";

import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from "@/components/ui/select";
import { cn } from "@/lib/utils";
import {
  countChanges,
  diffSnapshots,
  type Fields,
  type SectionDiff,
  type Snapshot,
  type SnapshotInfo,
} from "@/features/snapshots/diff";

const NOW = "now";

function errorMessage(err: unknown) {
  return err instanceof Error ? err.message : String(err);
}

function formatTime(ms: number) {
  return new Date(ms).toLocaleString(undefined, { dateStyle: "medium", timeStyle: "short" });
}

function snapshotName(t: TFunction, s: { label: string; takenAt: number }) {
  return s.label || t("snapshots.untitled", { time: formatTime(s.takenAt) });
}

function summarize(fields: Fields) {
  return Object.entries(fields)
    .map(([k, v]) => `${k}: ${v}`)
    .join(" · ");
}

function SectionResult({ section }: { section: SectionDiff }) {
  const { t } = useTranslation();
  const title = t(`snapshots.sections.${section.id}`, { defaultValue: section.id });
  return (
    <div className="flex flex-col gap-1.5">
      <h3 className="flex items-center gap-2 text-sm font-medium">
        {title}
        {!section.unreadable && (
          <Badge variant={section.changes.length ? "default" : "secondary"}>
            {t("snapshots.changeCount", { count: section.changes.length })}
          </Badge>
        )}
      </h3>
      {section.unreadable && (
        <p className="text-sm text-muted-foreground">{t(`snapshots.unreadable.${section.unreadable}`)}</p>
      )}
      {section.changes.map((change) => (
        <div key={change.kind + change.key} className="rounded-md border px-3 py-2 text-sm">
          <div className="flex items-start gap-2">
            <span
              className={cn(
                "w-4 shrink-0 font-mono font-bold",
                change.kind === "added" && "text-green-600 dark:text-green-400",
                change.kind === "removed" && "text-destructive",
                change.kind === "changed" && "text-amber-600 dark:text-amber-400",
              )}
              aria-label={t(`snapshots.kind.${change.kind}`)}
              title={t(`snapshots.kind.${change.kind}`)}
            >
              {change.kind === "added" ? "+" : change.kind === "removed" ? "−" : "~"}
            </span>
            <div className="min-w-0 flex-1">
              <div className="font-medium break-all">{change.key}</div>
              {change.kind !== "changed" ? (
                <div className="text-xs break-all text-muted-foreground">{summarize(change.fields)}</div>
              ) : (
                <ul className="mt-1 flex flex-col gap-0.5 text-xs">
                  {change.changes.map((c) => (
                    <li key={c.field} className="break-all">
                      <span className="text-muted-foreground">{c.field}: </span>
                      <span className="text-destructive line-through decoration-1">
                        {c.before ?? t("snapshots.none")}
                      </span>
                      <span className="text-muted-foreground"> → </span>
                      <span className="text-green-600 dark:text-green-400">{c.after ?? t("snapshots.none")}</span>
                    </li>
                  ))}
                </ul>
              )}
            </div>
          </div>
        </div>
      ))}
    </div>
  );
}

function RenameDialog({
  target,
  onClose,
  onRenamed,
}: {
  target: SnapshotInfo | null;
  onClose: () => void;
  onRenamed: () => void;
}) {
  const { t } = useTranslation();
  const [label, setLabel] = useState("");
  const [busy, setBusy] = useState(false);

  useEffect(() => {
    setLabel(target?.label ?? "");
  }, [target]);

  async function rename() {
    if (!target) return;
    setBusy(true);
    try {
      await invoke("rename_snapshot", { id: target.id, label });
      onClose();
      onRenamed();
    } catch (err) {
      toast.error(errorMessage(err));
    } finally {
      setBusy(false);
    }
  }

  return (
    <Dialog open={!!target} onOpenChange={(open) => !open && !busy && onClose()}>
      <DialogContent>
        <DialogHeader>
          <DialogTitle>{t("snapshots.renameTitle")}</DialogTitle>
          <DialogDescription>{target && formatTime(target.takenAt)}</DialogDescription>
        </DialogHeader>
        <Input
          value={label}
          maxLength={80}
          onChange={(e) => setLabel(e.currentTarget.value)}
          onKeyDown={(e) => e.key === "Enter" && rename()}
          aria-label={t("snapshots.nameLabel")}
          autoFocus
        />
        <DialogFooter>
          <Button variant="outline" onClick={onClose} disabled={busy}>
            {t("snapshots.cancel")}
          </Button>
          <Button onClick={rename} disabled={busy}>
            {t("snapshots.save")}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}

export function SnapshotsPage() {
  const { t } = useTranslation();
  const [list, setList] = useState<SnapshotInfo[]>([]);
  const [listError, setListError] = useState<string | null>(null);
  const [label, setLabel] = useState("");
  const [taking, setTaking] = useState(false);
  const [before, setBefore] = useState<string>("");
  const [after, setAfter] = useState<string>(NOW);
  const [comparing, setComparing] = useState(false);
  const [result, setResult] = useState<{ before: Snapshot; after: Snapshot; diff: SectionDiff[] } | null>(null);
  const [renaming, setRenaming] = useState<SnapshotInfo | null>(null);
  const [deleting, setDeleting] = useState<SnapshotInfo | null>(null);

  const refresh = useCallback(async () => {
    try {
      const items = await invoke<SnapshotInfo[]>("list_snapshots");
      setList(items);
      setListError(null);
      setBefore((current) => (items.some((s) => s.id === current) ? current : (items[0]?.id ?? "")));
      setAfter((current) => (current === NOW || items.some((s) => s.id === current) ? current : NOW));
    } catch (err) {
      setListError(errorMessage(err));
    }
  }, []);

  useEffect(() => {
    refresh();
  }, [refresh]);

  async function take() {
    setTaking(true);
    try {
      const info = await invoke<SnapshotInfo>("take_snapshot", { label });
      toast.success(t("snapshots.taken", { name: snapshotName(t, info) }));
      setLabel("");
      setBefore(info.id);
      await refresh();
    } catch (err) {
      toast.error(t("snapshots.takeError", { error: errorMessage(err) }));
    } finally {
      setTaking(false);
    }
  }

  async function load(id: string) {
    return id === NOW
      ? invoke<Snapshot>("current_snapshot")
      : invoke<Snapshot>("get_snapshot", { id });
  }

  async function compare() {
    if (!before) return;
    setComparing(true);
    try {
      const [a, b] = await Promise.all([load(before), load(after)]);
      setResult({ before: a, after: b, diff: diffSnapshots(a, b) });
    } catch (err) {
      setResult(null);
      toast.error(t("snapshots.compareError", { error: errorMessage(err) }));
    } finally {
      setComparing(false);
    }
  }

  async function confirmDelete() {
    if (!deleting) return;
    try {
      await invoke("delete_snapshot", { id: deleting.id });
      if (result && (result.before.id === deleting.id || result.after.id === deleting.id)) setResult(null);
      setDeleting(null);
      await refresh();
    } catch (err) {
      toast.error(errorMessage(err));
    }
  }

  const options: [string, string][] = list.map((s) => [s.id, `${snapshotName(t, s)} · ${formatTime(s.takenAt)}`]);
  const optionLabel = (value: string) =>
    value === NOW ? t("snapshots.now") : (options.find(([id]) => id === value)?.[1] ?? value);
  const total = result ? countChanges(result.diff) : 0;

  return (
    <div className="flex flex-col gap-6">
      <div>
        <h1 className="text-lg font-semibold">{t("snapshots.title")}</h1>
        <p className="text-sm text-muted-foreground">{t("snapshots.subtitle")}</p>
      </div>

      <Card>
        <CardHeader>
          <CardTitle>{t("snapshots.takeTitle")}</CardTitle>
          <CardDescription>{t("snapshots.takeDescription")}</CardDescription>
        </CardHeader>
        <CardContent className="flex flex-col gap-2 sm:flex-row sm:items-end">
          <div className="flex flex-1 flex-col gap-1.5">
            <Label htmlFor="snapshot-label">{t("snapshots.nameLabel")}</Label>
            <Input
              id="snapshot-label"
              value={label}
              maxLength={80}
              placeholder={t("snapshots.namePlaceholder")}
              onChange={(e) => setLabel(e.currentTarget.value)}
              onKeyDown={(e) => e.key === "Enter" && !taking && take()}
            />
          </div>
          <Button onClick={take} disabled={taking}>
            {taking ? <Loader2 className="animate-spin" /> : <Camera />}
            {taking ? t("snapshots.taking") : t("snapshots.take")}
          </Button>
        </CardContent>
      </Card>

      <Card>
        <CardHeader>
          <CardTitle>{t("snapshots.compareTitle")}</CardTitle>
          <CardDescription>{t("snapshots.compareDescription")}</CardDescription>
        </CardHeader>
        <CardContent className="flex flex-col gap-4">
          {list.length === 0 ? (
            <p className="text-sm text-muted-foreground">{t("snapshots.needOne")}</p>
          ) : (
            <div className="grid gap-2 sm:grid-cols-[1fr_1fr_auto] sm:items-end">
              <div className="flex flex-col gap-1.5">
                <Label>{t("snapshots.before")}</Label>
                <Select value={before} onValueChange={(v) => setBefore(v as string)}>
                  <SelectTrigger className="w-full">
                    <SelectValue>{(v: string) => optionLabel(v)}</SelectValue>
                  </SelectTrigger>
                  <SelectContent>
                    {options.map(([id, name]) => (
                      <SelectItem key={id} value={id}>
                        {name}
                      </SelectItem>
                    ))}
                  </SelectContent>
                </Select>
              </div>
              <div className="flex flex-col gap-1.5">
                <Label>{t("snapshots.after")}</Label>
                <Select value={after} onValueChange={(v) => setAfter(v as string)}>
                  <SelectTrigger className="w-full">
                    <SelectValue>{(v: string) => optionLabel(v)}</SelectValue>
                  </SelectTrigger>
                  <SelectContent>
                    <SelectItem value={NOW}>{t("snapshots.now")}</SelectItem>
                    {options.map(([id, name]) => (
                      <SelectItem key={id} value={id}>
                        {name}
                      </SelectItem>
                    ))}
                  </SelectContent>
                </Select>
              </div>
              <Button onClick={compare} disabled={comparing || !before || before === after}>
                {comparing ? <Loader2 className="animate-spin" /> : <GitCompareArrows />}
                {comparing ? t("snapshots.comparing") : t("snapshots.compare")}
              </Button>
            </div>
          )}

          {result && (
            <div className="flex flex-col gap-4 border-t pt-4">
              <p className={cn("text-sm font-medium", total === 0 && "text-green-600 dark:text-green-400")}>
                {total === 0 ? t("snapshots.noDifferences") : t("snapshots.differences", { count: total })}
              </p>
              {result.before.computer &&
                result.after.computer &&
                result.before.computer !== result.after.computer && (
                  <p className="text-sm text-amber-600 dark:text-amber-400">
                    {t("snapshots.otherComputer", {
                      before: result.before.computer,
                      after: result.after.computer,
                    })}
                  </p>
                )}
              {result.diff
                .filter((s) => s.changes.length || s.unreadable)
                .map((section) => (
                  <SectionResult key={section.id} section={section} />
                ))}
            </div>
          )}
        </CardContent>
      </Card>

      <Card>
        <CardHeader>
          <div className="flex items-center justify-between gap-2">
            <div className="flex flex-col gap-1">
              <CardTitle>{t("snapshots.savedTitle")}</CardTitle>
              <CardDescription>{t("snapshots.savedDescription")}</CardDescription>
            </div>
            <Button variant="ghost" size="icon-sm" onClick={refresh} aria-label={t("snapshots.refresh")}>
              <RefreshCw />
            </Button>
          </div>
        </CardHeader>
        <CardContent className="flex flex-col gap-2">
          {listError && <p className="text-sm text-destructive">{listError}</p>}
          {list.length === 0 && !listError && (
            <p className="text-sm text-muted-foreground">{t("snapshots.noneSaved")}</p>
          )}
          {list.map((s) => (
            <div key={s.id} className="flex items-center gap-3 rounded-md border px-3 py-2">
              <div className="min-w-0 flex-1">
                <div className="truncate text-sm font-medium">{snapshotName(t, s)}</div>
                <div className="text-xs text-muted-foreground">
                  {formatTime(s.takenAt)} · {t("snapshots.itemCount", { count: s.itemCount })}
                  {s.computer && ` · ${s.computer}`}
                  {s.errorCount > 0 && ` · ${t("snapshots.errorCount", { count: s.errorCount })}`}
                </div>
              </div>
              <Button
                variant="ghost"
                size="icon-sm"
                aria-label={t("snapshots.rename")}
                title={t("snapshots.rename")}
                onClick={() => setRenaming(s)}
              >
                <Pencil />
              </Button>
              <Button
                variant="ghost"
                size="icon-sm"
                aria-label={t("snapshots.delete")}
                title={t("snapshots.delete")}
                onClick={() => setDeleting(s)}
              >
                <Trash2 />
              </Button>
            </div>
          ))}
        </CardContent>
      </Card>

      <RenameDialog target={renaming} onClose={() => setRenaming(null)} onRenamed={refresh} />

      <Dialog open={!!deleting} onOpenChange={(open) => !open && setDeleting(null)}>
        <DialogContent>
          <DialogHeader>
            <DialogTitle>{t("snapshots.deleteTitle")}</DialogTitle>
            <DialogDescription>{deleting && snapshotName(t, deleting)}</DialogDescription>
          </DialogHeader>
          <DialogFooter>
            <Button variant="outline" onClick={() => setDeleting(null)}>
              {t("snapshots.cancel")}
            </Button>
            <Button variant="destructive" onClick={confirmDelete}>
              {t("snapshots.delete")}
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </div>
  );
}
