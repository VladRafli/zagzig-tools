import { useCallback, useEffect, useMemo, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { Loader2, Lock, Pencil, Plus, RefreshCw, Search, Trash2, Undo2 } from "lucide-react";
import { useTranslation } from "react-i18next";
import type { TFunction } from "i18next";
import { toast } from "sonner";

import { AdminRequiredTooltip } from "@/components/admin-required-tooltip";
import { CollapsibleDetails } from "@/components/collapsible-details";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Checkbox } from "@/components/ui/checkbox";
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
import { Textarea } from "@/components/ui/textarea";
import { joinList, ListEditor, splitList } from "@/features/environment/list-editor";
import {
  hasVariableReference,
  isPathList,
  isProtected,
  useEnvironment,
  type EnvChange,
  type EnvScope,
  type EnvVar,
} from "@/features/environment/use-environment";
import { formatRelativeTime } from "@/lib/relative-time";
import { useIsAdministrator } from "@/lib/use-is-administrator";

// Some older programs ignore anything past this many characters.
const LONG_VALUE = 2047;

function errorMessage(err: unknown) {
  return err instanceof Error ? err.message : String(err);
}

function VariableDialog({
  scope,
  names,
  existing,
  open,
  onClose,
  onSaved,
  t,
}: {
  scope: EnvScope;
  /** Names already in this scope, to stop "add" from overwriting one. */
  names: string[];
  /** `null` when adding a new variable. */
  existing: EnvVar | null;
  open: boolean;
  onClose: () => void;
  onSaved: () => void;
  t: TFunction;
}) {
  const [name, setName] = useState("");
  const [value, setValue] = useState("");
  const [asList, setAsList] = useState(false);
  const [expand, setExpand] = useState(false);
  const [expandTouched, setExpandTouched] = useState(false);
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState<string | null>(null);

  // Start fresh each time the dialog opens.
  useEffect(() => {
    if (!open) return;
    const initialName = existing?.name ?? "";
    const initialValue = existing?.value ?? "";
    setName(initialName);
    setValue(initialValue);
    setExpand(existing ? existing.kind === "expand" : false);
    setExpandTouched(existing !== null);
    setAsList(initialValue.includes(";") || isPathList(initialName));
    setError(null);
    // Keyed on the name, not the object, so a background refresh of the list
    // doesn't wipe what the user is typing.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [open, existing?.name]);

  const entries = useMemo(() => splitList(value), [value]);
  const editing = existing !== null;
  const duplicateName =
    !editing && names.some((n) => n.toLowerCase() === name.trim().toLowerCase());
  const canSave = name.trim().length > 0 && !duplicateName && !saving;

  function changeValue(next: string) {
    setValue(next);
    // A value that references %VARS% only works if stored as expandable —
    // switch it on unless the user decided otherwise.
    if (!expandTouched) setExpand(hasVariableReference(next));
  }

  async function save() {
    setSaving(true);
    setError(null);
    try {
      await invoke("set_env_variable", {
        scope,
        name: name.trim(),
        value: asList ? joinList(entries) : value,
        kind: expand ? "expand" : "string",
      });
      toast.success(t("environment.saveSuccess", { name: name.trim() }));
      onSaved();
      onClose();
    } catch (err) {
      setError(errorMessage(err));
    } finally {
      setSaving(false);
    }
  }

  const current = asList ? joinList(entries) : value;

  return (
    <Dialog open={open} onOpenChange={(next) => !next && !saving && onClose()}>
      <DialogContent className="sm:max-w-2xl">
        <DialogHeader>
          <DialogTitle>
            {editing ? t("environment.edit.title", { name }) : t("environment.add.title")}
          </DialogTitle>
          <DialogDescription>
            {scope === "system" ? t("environment.systemNote") : t("environment.userNote")}
          </DialogDescription>
        </DialogHeader>

        <div className="flex flex-col gap-4">
          <div className="flex flex-col gap-1.5">
            <Label htmlFor="envName">{t("environment.nameLabel")}</Label>
            <Input
              id="envName"
              value={name}
              onChange={(e) => setName(e.currentTarget.value)}
              disabled={editing || saving}
              placeholder="JAVA_HOME"
              className="font-mono text-sm"
            />
          </div>

          <div className="flex flex-col gap-1.5">
            <div className="flex items-center justify-between">
              <Label htmlFor="envValue">{t("environment.valueLabel")}</Label>
              <Button variant="ghost" size="sm" onClick={() => setAsList((v) => !v)} disabled={saving}>
                {asList ? t("environment.editAsText") : t("environment.editAsList")}
              </Button>
            </div>
            {asList ? (
              <ListEditor
                entries={entries}
                onChange={(next) => changeValue(next.join(";"))}
                checkFolders={isPathList(name)}
                disabled={saving}
                t={t}
              />
            ) : (
              <Textarea
                id="envValue"
                value={value}
                onChange={(e) => changeValue(e.currentTarget.value)}
                disabled={saving}
                spellCheck={false}
                className="min-h-24 font-mono text-xs"
              />
            )}
            <p
              className={`text-xs ${
                current.length > LONG_VALUE ? "text-amber-600 dark:text-amber-400" : "text-muted-foreground"
              }`}
            >
              {t("environment.length", { count: current.length })}
              {current.length > LONG_VALUE ? ` — ${t("environment.longWarning")}` : ""}
            </p>
          </div>

          <div className="flex items-start gap-2">
            <Checkbox
              id="envExpand"
              checked={expand}
              onCheckedChange={(c) => {
                setExpand(c === true);
                setExpandTouched(true);
              }}
              disabled={saving}
            />
            <Label htmlFor="envExpand" className="leading-snug">
              {t("environment.expandLabel")}
            </Label>
          </div>
          {duplicateName && (
            <p className="text-sm text-destructive">{t("environment.nameExists", { name: name.trim() })}</p>
          )}
          {error && <p className="text-sm text-destructive">{error}</p>}
        </div>

        <DialogFooter>
          <Button variant="outline" onClick={onClose} disabled={saving}>
            {t("environment.cancel")}
          </Button>
          <Button onClick={save} disabled={!canSave}>
            {saving && <Loader2 className="animate-spin" />}
            {saving ? t("environment.saving") : t("environment.save")}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}

function DeleteDialog({
  scope,
  variable,
  onClose,
  onDeleted,
  t,
}: {
  scope: EnvScope;
  variable: EnvVar | null;
  onClose: () => void;
  onDeleted: () => void;
  t: TFunction;
}) {
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);

  async function confirm() {
    if (!variable) return;
    setBusy(true);
    setError(null);
    try {
      await invoke("delete_env_variable", { scope, name: variable.name });
      toast.success(t("environment.deleteSuccess", { name: variable.name }));
      onDeleted();
      onClose();
    } catch (err) {
      setError(errorMessage(err));
    } finally {
      setBusy(false);
    }
  }

  return (
    <Dialog
      open={variable !== null}
      onOpenChange={(open) => {
        if (open || busy) return;
        setError(null);
        onClose();
      }}
    >
      <DialogContent>
        <DialogHeader>
          <DialogTitle>{t("environment.delete.title", { name: variable?.name ?? "" })}</DialogTitle>
          <DialogDescription>{t("environment.delete.description")}</DialogDescription>
        </DialogHeader>
        {error && <p className="text-sm text-destructive">{error}</p>}
        <DialogFooter>
          <Button variant="outline" onClick={onClose} disabled={busy}>
            {t("environment.cancel")}
          </Button>
          <Button variant="destructive" onClick={confirm} disabled={busy}>
            {busy && <Loader2 className="animate-spin" />}
            {t("environment.delete.confirm")}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}

const GRID = "grid-cols-[minmax(8rem,1fr)_2fr_6rem_auto]";

function summarize(variable: EnvVar, t: TFunction): string {
  if (variable.value.includes(";") && isPathList(variable.name)) {
    return t("environment.entries", { count: splitList(variable.value).length });
  }
  return variable.value;
}

function History({
  history,
  isAdministrator,
  onUndone,
  t,
}: {
  history: EnvChange[];
  isAdministrator: boolean;
  onUndone: () => void;
  t: TFunction;
}) {
  const [busyId, setBusyId] = useState<string | null>(null);

  async function undo(change: EnvChange) {
    setBusyId(change.id);
    try {
      await invoke("undo_env_change", { id: change.id });
      toast.success(t("environment.history.undone", { name: change.name }));
      onUndone();
    } catch (err) {
      toast.error(t("environment.history.undoError", { error: errorMessage(err) }));
    } finally {
      setBusyId(null);
    }
  }

  if (history.length === 0) {
    return <p className="mt-3 text-sm text-muted-foreground">{t("environment.history.none")}</p>;
  }

  return (
    <div className="mt-3 flex flex-col gap-2">
      <p className="text-sm text-muted-foreground">{t("environment.history.description")}</p>
      <div className="rounded-lg border">
        {history.map((change) => {
          const locked = change.scope === "system" && !isAdministrator;
          return (
            <div
              key={change.id}
              className="flex items-center gap-2 border-b px-3 py-2 text-sm last:border-b-0"
            >
              <div className="min-w-0 flex-1">
                <div className="flex flex-wrap items-center gap-2">
                  <span className="font-medium break-all">{change.name}</span>
                  <Badge variant="outline">
                    {change.scope === "system" ? t("environment.scope.system") : t("environment.scope.user")}
                  </Badge>
                  <span className="text-xs text-muted-foreground">
                    {t(`environment.history.actions.${change.action}`)} ·{" "}
                    {new Date(change.time * 1000).toLocaleString()}
                  </span>
                </div>
                <p className="truncate font-mono text-xs text-muted-foreground" title={change.previous?.value}>
                  {change.previous
                    ? t("environment.history.was", { value: change.previous.value })
                    : t("environment.history.didNotExist")}
                </p>
              </div>
              <AdminRequiredTooltip locked={locked}>
                <Button
                  variant="outline"
                  size="sm"
                  disabled={locked || busyId !== null}
                  onClick={() => undo(change)}
                >
                  {locked ? <Lock /> : busyId === change.id ? <Loader2 className="animate-spin" /> : <Undo2 />}
                  {t("environment.history.undo")}
                </Button>
              </AdminRequiredTooltip>
            </div>
          );
        })}
      </div>
    </div>
  );
}

export function EnvironmentPage() {
  const { t } = useTranslation();
  const env = useEnvironment();
  const { isAdministrator } = useIsAdministrator();

  const [scope, setScope] = useState<EnvScope>("user");
  const [query, setQuery] = useState("");
  const [editing, setEditing] = useState<EnvVar | null>(null);
  const [adding, setAdding] = useState(false);
  const [deleting, setDeleting] = useState<EnvVar | null>(null);
  const [history, setHistory] = useState<EnvChange[]>([]);

  const loadHistory = useCallback(() => {
    invoke<EnvChange[]>("get_env_history")
      .then(setHistory)
      .catch(() => setHistory([]));
  }, []);

  useEffect(() => {
    loadHistory();
  }, [loadHistory]);

  // A change refreshes both the list and the history.
  const changed = useCallback(() => {
    env.refresh();
    loadHistory();
  }, [env, loadHistory]);

  const locked = scope === "system" && !isAdministrator;
  const variables = scope === "user" ? env.user : env.system;

  const rows = useMemo(() => {
    const needle = query.trim().toLowerCase();
    return variables.filter(
      (v) => !needle || v.name.toLowerCase().includes(needle) || v.value.toLowerCase().includes(needle),
    );
  }, [variables, query]);

  return (
    <div className="flex flex-col gap-6">
      <div>
        <h1 className="text-lg font-semibold">{t("environment.title")}</h1>
        <p className="text-sm text-muted-foreground">{t("environment.subtitle")}</p>
      </div>

      <p className="rounded-lg border bg-muted/30 px-3 py-2 text-sm text-muted-foreground">
        {t("environment.restartNote")}
      </p>

      <div className="flex flex-col gap-3">
        <div className="flex flex-wrap items-center gap-2">
          <div className="flex rounded-lg border p-0.5">
            {(["user", "system"] as EnvScope[]).map((s) => (
              <Button
                key={s}
                size="sm"
                variant={scope === s ? "default" : "ghost"}
                onClick={() => setScope(s)}
              >
                {t(`environment.scope.${s}`)} ({s === "user" ? env.user.length : env.system.length})
              </Button>
            ))}
          </div>
          <div className="relative min-w-48 flex-1">
            <Search className="pointer-events-none absolute top-1/2 left-2.5 size-4 -translate-y-1/2 text-muted-foreground" />
            <Input
              className="pl-8"
              placeholder={t("environment.searchPlaceholder")}
              value={query}
              onChange={(e) => setQuery(e.currentTarget.value)}
            />
          </div>
          <AdminRequiredTooltip locked={locked}>
            <Button onClick={() => setAdding(true)} disabled={locked}>
              {locked ? <Lock /> : <Plus />}
              {t("environment.add.button")}
            </Button>
          </AdminRequiredTooltip>
          <div className="flex items-center gap-2">
            {env.updatedAt && (
              <span className="text-xs text-muted-foreground">
                {t("common.updatedAgo", { time: formatRelativeTime(t, env.updatedAt) })}
              </span>
            )}
            <Button variant="ghost" size="icon-sm" onClick={changed} disabled={env.status === "loading"}>
              <RefreshCw className={env.status === "loading" ? "animate-spin" : ""} />
            </Button>
          </div>
        </div>

        {env.status === "loading" && variables.length === 0 && (
          <p className="flex items-center gap-2 text-sm text-muted-foreground">
            <Loader2 className="size-4 animate-spin" />
            {t("environment.reading")}
          </p>
        )}
        {env.status === "error" && variables.length === 0 && (
          <p className="text-sm text-destructive">{t("environment.couldntRead", { error: env.error })}</p>
        )}
        {env.status === "ready" && rows.length === 0 && (
          <p className="text-sm text-muted-foreground">{t("environment.none")}</p>
        )}
        {rows.length > 0 && (
          <div className="overflow-x-auto rounded-lg border">
            <div className="min-w-[40rem]">
              <div
                className={`grid ${GRID} gap-2 border-b bg-muted/50 px-3 py-2 text-xs font-medium text-muted-foreground`}
              >
                <span>{t("environment.columns.name")}</span>
                <span>{t("environment.columns.value")}</span>
                <span>{t("environment.columns.type")}</span>
                <span />
              </div>
              {rows.map((v) => (
                <div
                  key={v.name}
                  className={`grid ${GRID} items-center gap-2 border-b px-3 py-2 text-sm last:border-b-0`}
                >
                  <span className="font-mono text-xs font-medium break-all">{v.name}</span>
                  <span className="truncate font-mono text-xs text-muted-foreground" title={v.value}>
                    {summarize(v, t) || t("common.none")}
                  </span>
                  <span>
                    {v.kind === "expand" && (
                      <Badge variant="outline" title={t("environment.expandHint")}>
                        {t("environment.expandable")}
                      </Badge>
                    )}
                  </span>
                  <div className="flex justify-end gap-1">
                    <AdminRequiredTooltip locked={locked}>
                      <Button
                        variant="ghost"
                        size="icon-sm"
                        aria-label={t("environment.edit.button")}
                        disabled={locked}
                        onClick={() => setEditing(v)}
                      >
                        {locked ? <Lock /> : <Pencil />}
                      </Button>
                    </AdminRequiredTooltip>
                    <AdminRequiredTooltip locked={locked}>
                      <Button
                        variant="ghost"
                        size="icon-sm"
                        aria-label={t("environment.delete.button")}
                        title={isProtected(scope, v.name) ? t("environment.protectedHint") : undefined}
                        disabled={locked || isProtected(scope, v.name)}
                        onClick={() => setDeleting(v)}
                      >
                        {locked ? <Lock /> : <Trash2 />}
                      </Button>
                    </AdminRequiredTooltip>
                  </div>
                </div>
              ))}
            </div>
          </div>
        )}
      </div>

      <CollapsibleDetails label={t("environment.history.label")}>
        <History history={history} isAdministrator={isAdministrator} onUndone={changed} t={t} />
      </CollapsibleDetails>

      <VariableDialog
        scope={scope}
        names={variables.map((v) => v.name)}
        existing={editing}
        open={adding || editing !== null}
        onClose={() => {
          setAdding(false);
          setEditing(null);
        }}
        onSaved={changed}
        t={t}
      />
      <DeleteDialog scope={scope} variable={deleting} onClose={() => setDeleting(null)} onDeleted={changed} t={t} />
    </div>
  );
}
