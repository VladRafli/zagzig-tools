import { useState } from "react";
import { BookmarkPlus, ListX, Trash2 } from "lucide-react";
import { useTranslation } from "react-i18next";

import { Button } from "@/components/ui/button";
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
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { useSavedTargets, type SavedTarget } from "@/lib/use-saved-targets";

function describe(target: SavedTarget): string {
  const extras = [target.ports, target.tlsPort ? `TLS ${target.tlsPort}` : undefined].filter(Boolean);
  return extras.length > 0 ? `${target.host} · ${extras.join(" · ")}` : target.host;
}

/**
 * A "saved hosts" picker with a Save button, shared by the pages that take a
 * host. `current` is what the page would save right now; `onLoad` fills the
 * page's form from a saved entry.
 */
export function SavedTargets({
  current,
  onLoad,
}: {
  current: Omit<SavedTarget, "id" | "name">;
  onLoad: (target: SavedTarget) => void;
}) {
  const { t } = useTranslation();
  const { targets, save, remove } = useSavedTargets();
  const [saving, setSaving] = useState(false);
  const [managing, setManaging] = useState(false);
  const [name, setName] = useState("");

  const canSave = current.host.trim().length > 0;

  function openSave() {
    setName(current.host.trim());
    setSaving(true);
  }

  function confirmSave() {
    if (!name.trim()) return;
    save({ ...current, name: name.trim() });
    setSaving(false);
  }

  return (
    <div className="flex flex-wrap items-center gap-2">
      <Select
        value=""
        onValueChange={(id) => {
          const target = targets.find((x) => x.id === id);
          if (target) onLoad(target);
        }}
        disabled={targets.length === 0}
      >
        <SelectTrigger size="sm" className="w-64" aria-label={t("savedTargets.label")}>
          <SelectValue placeholder={targets.length === 0 ? t("savedTargets.none") : t("savedTargets.pick")} />
        </SelectTrigger>
        <SelectContent>
          {targets.map((target) => (
            <SelectItem key={target.id} value={target.id}>
              <span className="font-medium">{target.name}</span>
              <span className="ml-2 text-xs text-muted-foreground">{describe(target)}</span>
            </SelectItem>
          ))}
        </SelectContent>
      </Select>
      <Button variant="outline" size="sm" onClick={openSave} disabled={!canSave}>
        <BookmarkPlus />
        {t("savedTargets.save")}
      </Button>
      {targets.length > 0 && (
        <Button variant="ghost" size="sm" onClick={() => setManaging(true)}>
          <ListX />
          {t("savedTargets.manage")}
        </Button>
      )}

      <Dialog open={saving} onOpenChange={setSaving}>
        <DialogContent>
          <DialogHeader>
            <DialogTitle>{t("savedTargets.saveTitle")}</DialogTitle>
            <DialogDescription>
              {t("savedTargets.saveDescription", { host: current.host.trim() })}
            </DialogDescription>
          </DialogHeader>
          <div className="flex flex-col gap-1.5">
            <Label htmlFor="savedTargetName">{t("savedTargets.nameLabel")}</Label>
            <Input
              id="savedTargetName"
              value={name}
              onChange={(e) => setName(e.currentTarget.value)}
              onKeyDown={(e) => e.key === "Enter" && confirmSave()}
              autoFocus
            />
            {targets.some((x) => x.name.toLowerCase() === name.trim().toLowerCase()) && (
              <p className="text-xs text-muted-foreground">{t("savedTargets.willUpdate")}</p>
            )}
          </div>
          <DialogFooter>
            <Button variant="outline" onClick={() => setSaving(false)}>
              {t("savedTargets.cancel")}
            </Button>
            <Button onClick={confirmSave} disabled={!name.trim()}>
              {t("savedTargets.saveConfirm")}
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>

      <Dialog open={managing} onOpenChange={setManaging}>
        <DialogContent>
          <DialogHeader>
            <DialogTitle>{t("savedTargets.manageTitle")}</DialogTitle>
            <DialogDescription>{t("savedTargets.manageDescription")}</DialogDescription>
          </DialogHeader>
          <div className="max-h-72 overflow-y-auto rounded-lg border">
            {targets.length === 0 && <p className="p-3 text-sm text-muted-foreground">{t("savedTargets.none")}</p>}
            {targets.map((target) => (
              <div key={target.id} className="flex items-center gap-2 border-b px-3 py-2 text-sm last:border-b-0">
                <div className="min-w-0 flex-1">
                  <div className="font-medium break-all">{target.name}</div>
                  <div className="truncate text-xs text-muted-foreground">{describe(target)}</div>
                </div>
                <Button
                  variant="ghost"
                  size="icon-sm"
                  aria-label={t("savedTargets.remove")}
                  onClick={() => remove(target.id)}
                >
                  <Trash2 />
                </Button>
              </div>
            ))}
          </div>
          <DialogFooter>
            <Button variant="outline" onClick={() => setManaging(false)}>
              {t("savedTargets.close")}
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </div>
  );
}
