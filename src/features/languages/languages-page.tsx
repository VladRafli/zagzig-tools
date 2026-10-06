import { useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { open, save } from "@tauri-apps/plugin-dialog";
import { Check, FileDown, FolderOpen, Loader2, Plus, RefreshCw, Trash2 } from "lucide-react";
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
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import {
  BUILT_IN_RESOURCES,
  getLanguagesSnapshot,
  loadLanguagePacks,
  setLanguage,
  useLanguages,
  usePackErrors,
  type LanguageInfo,
} from "@/i18n";
import { buildTemplate } from "@/i18n/pack";

function errorMessage(err: unknown) {
  return err instanceof Error ? err.message : String(err);
}

const GRID = "grid-cols-[1.4fr_6rem_8rem_auto]";

function LanguageRow({
  language,
  active,
  t,
  onRemove,
}: {
  language: LanguageInfo;
  active: boolean;
  t: TFunction;
  onRemove: (language: LanguageInfo) => void;
}) {
  const percent = Math.round(language.coverage * 100);
  const issues = language.mismatched + language.unknown;
  return (
    <div className={`grid ${GRID} items-center gap-2 border-b px-3 py-2 text-sm last:border-b-0`}>
      <div className="min-w-0">
        <div className="flex flex-wrap items-center gap-2">
          <span className="font-medium">{language.name}</span>
          <code className="text-xs text-muted-foreground">{language.code}</code>
          {active && <Badge variant="default">{t("languages.inUse")}</Badge>}
        </div>
        {issues > 0 && (
          <p className="mt-0.5 text-xs text-amber-600 dark:text-amber-400">
            {t("languages.issues", { mismatched: language.mismatched, unknown: language.unknown })}
          </p>
        )}
      </div>
      <Badge variant={language.builtIn ? "secondary" : "outline"} className="w-fit">
        {language.builtIn ? t("languages.builtIn") : t("languages.custom")}
      </Badge>
      <div className="flex flex-col gap-1">
        <span className="text-xs text-muted-foreground">{t("languages.coverage", { percent })}</span>
        <div className="h-1.5 overflow-hidden rounded-full bg-muted">
          <div className="h-full bg-primary" style={{ width: `${percent}%` }} />
        </div>
      </div>
      <div className="flex justify-end gap-1">
        <Button
          variant="outline"
          size="sm"
          disabled={active}
          onClick={() => setLanguage(language.code)}
        >
          {active && <Check />}
          {t("languages.use")}
        </Button>
        {!language.builtIn && (
          <Button
            variant="ghost"
            size="icon-sm"
            aria-label={t("languages.remove")}
            title={t("languages.remove")}
            onClick={() => onRemove(language)}
          >
            <Trash2 />
          </Button>
        )}
      </div>
    </div>
  );
}

export function LanguagesPage() {
  const { t, i18n } = useTranslation();
  const languages = useLanguages();
  const errors = usePackErrors();

  const [busy, setBusy] = useState<"add" | "reload" | null>(null);
  const [templateBase, setTemplateBase] = useState("en");
  const [removing, setRemoving] = useState<LanguageInfo | null>(null);

  async function addLanguage() {
    setBusy("add");
    try {
      const picked = await open({
        multiple: false,
        directory: false,
        filters: [{ name: t("languages.fileFilter"), extensions: ["json"] }],
      });
      if (typeof picked !== "string") return;
      const code = await invoke<string>("import_language_pack", { path: picked });
      await loadLanguagePacks();
      const info = getLanguagesSnapshot().find((l) => l.code === code);
      toast.success(t("languages.addSuccess", { name: info?.name ?? code }));
      if (info && (info.mismatched > 0 || info.unknown > 0)) {
        toast.warning(
          t("languages.issues", { mismatched: info.mismatched, unknown: info.unknown }),
        );
      }
    } catch (err) {
      toast.error(t("languages.addError", { error: errorMessage(err) }));
    } finally {
      setBusy(null);
    }
  }

  async function reload() {
    setBusy("reload");
    try {
      await loadLanguagePacks();
    } finally {
      setBusy(null);
    }
  }

  async function exportTemplate() {
    try {
      const target = await save({
        defaultPath: `${templateBase}-template.json`,
        filters: [{ name: t("languages.fileFilter"), extensions: ["json"] }],
      });
      if (!target) return;
      await invoke("save_language_template", {
        path: target,
        content: buildTemplate(BUILT_IN_RESOURCES[templateBase]),
      });
      toast.success(t("languages.exportSuccess"));
    } catch (err) {
      toast.error(t("languages.exportError", { error: errorMessage(err) }));
    }
  }

  async function confirmRemove() {
    const language = removing;
    if (!language) return;
    setRemoving(null);
    try {
      await invoke("remove_language_pack", { code: language.code });
      await loadLanguagePacks();
      toast.success(t("languages.removeSuccess", { name: language.name }));
    } catch (err) {
      toast.error(t("languages.removeError", { error: errorMessage(err) }));
    }
  }

  return (
    <div className="flex flex-col gap-6">
      <div>
        <h1 className="text-lg font-semibold">{t("languages.title")}</h1>
        <p className="text-sm text-muted-foreground">{t("languages.subtitle")}</p>
      </div>

      <div className="flex flex-col gap-3">
        <div className="flex flex-wrap items-center gap-2">
          <Button onClick={addLanguage} disabled={busy !== null}>
            {busy === "add" ? <Loader2 className="animate-spin" /> : <Plus />}
            {t("languages.add")}
          </Button>
          <Button variant="outline" onClick={() => invoke("reveal_languages_folder")}>
            <FolderOpen />
            {t("languages.openFolder")}
          </Button>
          <Button
            variant="ghost"
            size="icon-sm"
            aria-label={t("languages.reload")}
            title={t("languages.reload")}
            onClick={reload}
            disabled={busy !== null}
          >
            <RefreshCw className={busy === "reload" ? "animate-spin" : ""} />
          </Button>
        </div>

        <div className="overflow-x-auto rounded-lg border">
          <div className="min-w-[36rem]">
            <div
              className={`grid ${GRID} gap-2 border-b bg-muted/50 px-3 py-2 text-xs font-medium text-muted-foreground`}
            >
              <span>{t("languages.columns.language")}</span>
              <span>{t("languages.columns.type")}</span>
              <span>{t("languages.columns.coverage")}</span>
              <span />
            </div>
            {languages.map((language) => (
              <LanguageRow
                key={language.code}
                language={language}
                active={i18n.language === language.code}
                t={t}
                onRemove={setRemoving}
              />
            ))}
          </div>
        </div>

        {errors.length > 0 && (
          <div className="flex flex-col gap-1 rounded-lg border border-destructive/40 p-3 text-sm">
            <p className="font-medium text-destructive">{t("languages.loadErrors")}</p>
            {errors.map((e) => (
              <p key={e.file} className="text-xs text-muted-foreground">
                <code>{e.file}</code> — {e.error}
              </p>
            ))}
          </div>
        )}
      </div>

      <Card>
        <CardHeader>
          <CardTitle>{t("languages.howTo.title")}</CardTitle>
          <CardDescription>{t("languages.howTo.description")}</CardDescription>
        </CardHeader>
        <CardContent className="flex flex-col gap-4">
          <ol className="list-decimal space-y-1.5 pl-5 text-sm leading-relaxed">
            <li>{t("languages.howTo.step1")}</li>
            <li>{t("languages.howTo.step2")}</li>
            <li>{t("languages.howTo.step3", { example: "{{name}}" })}</li>
            <li>{t("languages.howTo.step4")}</li>
          </ol>
          <p className="text-xs text-muted-foreground">{t("languages.howTo.note")}</p>
          <div className="flex flex-wrap items-center gap-2">
            <Select value={templateBase} onValueChange={(v) => setTemplateBase(v as string)}>
              <SelectTrigger size="sm" className="w-52" aria-label={t("languages.templateFrom")}>
                <SelectValue>
                  {(v: string) =>
                    t("languages.templateOption", {
                      name: languages.find((l) => l.code === v)?.name ?? v,
                    })
                  }
                </SelectValue>
              </SelectTrigger>
              <SelectContent>
                {languages
                  .filter((l) => l.builtIn)
                  .map((l) => (
                    <SelectItem key={l.code} value={l.code}>
                      {t("languages.templateOption", { name: l.name })}
                    </SelectItem>
                  ))}
              </SelectContent>
            </Select>
            <Button variant="outline" onClick={exportTemplate}>
              <FileDown />
              {t("languages.exportTemplate")}
            </Button>
          </div>
        </CardContent>
      </Card>

      <Dialog open={removing !== null} onOpenChange={(o) => !o && setRemoving(null)}>
        <DialogContent>
          <DialogHeader>
            <DialogTitle>{t("languages.removeTitle", { name: removing?.name ?? "" })}</DialogTitle>
            <DialogDescription>{t("languages.removeDescription")}</DialogDescription>
          </DialogHeader>
          <DialogFooter>
            <Button variant="outline" onClick={() => setRemoving(null)}>
              {t("languages.cancel")}
            </Button>
            <Button variant="destructive" onClick={confirmRemove}>
              {t("languages.removeConfirm")}
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </div>
  );
}
