import { useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { save } from "@tauri-apps/plugin-dialog";
import { ClipboardCopy, ClipboardList, Download, Loader2 } from "lucide-react";
import { useTranslation } from "react-i18next";
import { toast } from "sonner";

import { Button } from "@/components/ui/button";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { Checkbox } from "@/components/ui/checkbox";
import { Label } from "@/components/ui/label";
import { Textarea } from "@/components/ui/textarea";

const SECTIONS = [
  "system",
  "adapters",
  "dns",
  "nrpt",
  "routes",
  "proxy",
  "hosts",
  "ports",
  "firewall",
  "wsl",
  "events",
] as const;
type SectionId = (typeof SECTIONS)[number];

function errorMessage(err: unknown) {
  return err instanceof Error ? err.message : String(err);
}

export function DiagnosticsPage() {
  const { t } = useTranslation();
  const [selected, setSelected] = useState<Set<SectionId>>(new Set(SECTIONS));
  const [hidePersonal, setHidePersonal] = useState(true);
  const [maskIps, setMaskIps] = useState(false);
  const [building, setBuilding] = useState(false);
  const [report, setReport] = useState<string | null>(null);
  const [failure, setFailure] = useState<string | null>(null);

  function toggle(id: SectionId, on: boolean) {
    setSelected((prev) => {
      const next = new Set(prev);
      if (on) next.add(id);
      else next.delete(id);
      return next;
    });
  }

  async function build() {
    setBuilding(true);
    setFailure(null);
    try {
      setReport(
        await invoke<string>("build_diagnostic_report", {
          options: { sections: [...selected], hidePersonal, maskIps },
        }),
      );
    } catch (err) {
      setReport(null);
      setFailure(errorMessage(err));
    } finally {
      setBuilding(false);
    }
  }

  async function copy() {
    if (!report) return;
    try {
      await navigator.clipboard.writeText(report);
      toast.success(t("diagnostics.copied"));
    } catch {
      toast.error(t("diagnostics.copyFailed"));
    }
  }

  async function saveFile() {
    if (!report) return;
    try {
      const stamp = new Date().toISOString().slice(0, 10);
      const target = await save({
        defaultPath: `zagzig-diagnostics-${stamp}.md`,
        filters: [{ name: t("diagnostics.fileFilter"), extensions: ["md", "txt"] }],
      });
      if (!target) return;
      await invoke("save_text_report", { path: target, content: report });
      toast.success(t("diagnostics.saved"));
    } catch (err) {
      toast.error(t("diagnostics.saveError", { error: errorMessage(err) }));
    }
  }

  return (
    <div className="flex flex-col gap-6">
      <div>
        <h1 className="text-lg font-semibold">{t("diagnostics.title")}</h1>
        <p className="text-sm text-muted-foreground">{t("diagnostics.subtitle")}</p>
      </div>

      <Card>
        <CardHeader>
          <CardTitle>{t("diagnostics.include")}</CardTitle>
          <CardDescription>{t("diagnostics.includeDescription")}</CardDescription>
        </CardHeader>
        <CardContent className="flex flex-col gap-4">
          <div className="grid gap-x-6 gap-y-2 sm:grid-cols-2 lg:grid-cols-3">
            {SECTIONS.map((id) => (
              <div key={id} className="flex items-start gap-2">
                <Checkbox
                  id={`diag-${id}`}
                  checked={selected.has(id)}
                  onCheckedChange={(c) => toggle(id, c === true)}
                  disabled={building}
                />
                <Label htmlFor={`diag-${id}`} className="flex flex-col items-start gap-0.5 leading-snug">
                  <span>{t(`diagnostics.sections.${id}.title`)}</span>
                  <span className="text-xs font-normal text-muted-foreground">
                    {t(`diagnostics.sections.${id}.detail`)}
                  </span>
                </Label>
              </div>
            ))}
          </div>

          <div className="flex flex-col gap-2 border-t pt-4">
            <div className="flex items-start gap-2">
              <Checkbox
                id="diag-personal"
                checked={hidePersonal}
                onCheckedChange={(c) => setHidePersonal(c === true)}
                disabled={building}
              />
              <Label htmlFor="diag-personal" className="flex flex-col items-start gap-0.5 leading-snug">
                <span>{t("diagnostics.hidePersonal")}</span>
                <span className="text-xs font-normal text-muted-foreground">{t("diagnostics.hidePersonalHint")}</span>
              </Label>
            </div>
            <div className="flex items-start gap-2">
              <Checkbox
                id="diag-ips"
                checked={maskIps}
                onCheckedChange={(c) => setMaskIps(c === true)}
                disabled={building}
              />
              <Label htmlFor="diag-ips" className="flex flex-col items-start gap-0.5 leading-snug">
                <span>{t("diagnostics.maskIps")}</span>
                <span className="text-xs font-normal text-muted-foreground">{t("diagnostics.maskIpsHint")}</span>
              </Label>
            </div>
          </div>

          <div>
            <Button onClick={build} disabled={building || selected.size === 0}>
              {building ? <Loader2 className="animate-spin" /> : <ClipboardList />}
              {building ? t("diagnostics.building") : report ? t("diagnostics.rebuild") : t("diagnostics.build")}
            </Button>
          </div>
        </CardContent>
      </Card>

      {failure && <p className="text-sm text-destructive">{failure}</p>}

      {report && (
        <div className="flex flex-col gap-3">
          <div className="flex flex-wrap items-center gap-2">
            <Button variant="outline" onClick={copy}>
              <ClipboardCopy />
              {t("diagnostics.copy")}
            </Button>
            <Button variant="outline" onClick={saveFile}>
              <Download />
              {t("diagnostics.save")}
            </Button>
            <span className="text-xs text-muted-foreground">{t("diagnostics.reviewNote")}</span>
          </div>
          <Textarea
            readOnly
            value={report}
            spellCheck={false}
            className="min-h-96 font-mono text-xs"
            aria-label={t("diagnostics.preview")}
          />
        </div>
      )}
    </div>
  );
}
