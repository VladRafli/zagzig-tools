import { useState, type FormEvent } from "react";
import { invoke } from "@tauri-apps/api/core";
import { Loader2, ScanSearch, Square } from "lucide-react";
import { useTranslation } from "react-i18next";

import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";

interface OpenPort {
  port: number;
  service: string | null;
}

interface ScanResult {
  host: string;
  address: string;
  open: OpenPort[];
  scanned: number;
  closed: number;
  filtered: number;
  errors: number;
  cancelled: boolean;
  durationMs: number;
}

const PRESETS: Record<string, string> = {
  common: "21,22,23,25,53,80,110,135,139,143,443,445,993,995,1433,3306,3389,5432,5900,8080",
  web: "80,443,3000,5000,8000,8008,8080,8443,8888",
  databases: "1433,1521,3306,5432,6379,9200,11211,27017",
  dev: "3000,3001,4200,5000,5173,5432,6379,8000,8080,8081,8888,9000",
  wellKnown: "1-1024",
};
const PRESET_IDS = [...Object.keys(PRESETS), "custom"];
const TIMEOUTS = [300, 800, 2000, 4000];

export function PortScannerPage() {
  const { t } = useTranslation();
  const [host, setHost] = useState("");
  const [preset, setPreset] = useState("common");
  const [custom, setCustom] = useState("");
  const [timeoutMs, setTimeoutMs] = useState(800);
  const [scanning, setScanning] = useState(false);
  const [result, setResult] = useState<ScanResult | null>(null);
  const [failure, setFailure] = useState<string | null>(null);

  const ports = preset === "custom" ? custom : PRESETS[preset];
  const canScan = host.trim().length > 0 && ports.trim().length > 0 && !scanning;

  async function scan(e?: FormEvent) {
    e?.preventDefault();
    if (!canScan) return;
    setScanning(true);
    setFailure(null);
    setResult(null);
    try {
      setResult(await invoke<ScanResult>("scan_ports", { host, ports, timeoutMs }));
    } catch (err) {
      setFailure(err instanceof Error ? err.message : String(err));
    } finally {
      setScanning(false);
    }
  }

  return (
    <div className="flex flex-col gap-6">
      <div>
        <h1 className="text-lg font-semibold">{t("portScanner.title")}</h1>
        <p className="text-sm text-muted-foreground">{t("portScanner.subtitle")}</p>
      </div>

      <p className="rounded-lg border bg-muted/30 px-3 py-2 text-sm text-muted-foreground">
        {t("portScanner.permission")}
      </p>

      <form onSubmit={scan} className="flex flex-col gap-4">
        <div className="grid gap-4 sm:grid-cols-[2fr_1.5fr_1fr]">
          <div className="flex flex-col gap-1.5">
            <Label htmlFor="scanHost">{t("portScanner.hostLabel")}</Label>
            <Input
              id="scanHost"
              value={host}
              onChange={(e) => setHost(e.currentTarget.value)}
              placeholder="192.168.1.10 or server.local"
              disabled={scanning}
            />
          </div>
          <div className="flex flex-col gap-1.5">
            <Label htmlFor="scanPreset">{t("portScanner.portsLabel")}</Label>
            <Select value={preset} onValueChange={(v) => setPreset(v as string)} disabled={scanning}>
              <SelectTrigger id="scanPreset" className="w-full">
                <SelectValue>{(v: string) => t(`portScanner.presets.${v}`)}</SelectValue>
              </SelectTrigger>
              <SelectContent>
                {PRESET_IDS.map((id) => (
                  <SelectItem key={id} value={id}>
                    {t(`portScanner.presets.${id}`)}
                  </SelectItem>
                ))}
              </SelectContent>
            </Select>
          </div>
          <div className="flex flex-col gap-1.5">
            <Label htmlFor="scanTimeout">{t("portScanner.timeoutLabel")}</Label>
            <Select value={String(timeoutMs)} onValueChange={(v) => setTimeoutMs(Number(v))} disabled={scanning}>
              <SelectTrigger id="scanTimeout" className="w-full">
                <SelectValue>{(v: string) => `${v} ms`}</SelectValue>
              </SelectTrigger>
              <SelectContent>
                {TIMEOUTS.map((ms) => (
                  <SelectItem key={ms} value={String(ms)}>
                    {ms} ms
                  </SelectItem>
                ))}
              </SelectContent>
            </Select>
          </div>
        </div>
        {preset === "custom" ? (
          <div className="flex flex-col gap-1.5">
            <Label htmlFor="scanCustom">{t("portScanner.customLabel")}</Label>
            <Input
              id="scanCustom"
              value={custom}
              onChange={(e) => setCustom(e.currentTarget.value)}
              placeholder="22,80,443,8000-8100"
              className="font-mono"
              disabled={scanning}
            />
            <p className="text-xs text-muted-foreground">{t("portScanner.customHint")}</p>
          </div>
        ) : (
          <p className="break-all font-mono text-xs text-muted-foreground">{ports}</p>
        )}
        <div className="flex gap-2">
          <Button type="submit" disabled={!canScan}>
            {scanning ? <Loader2 className="animate-spin" /> : <ScanSearch />}
            {scanning ? t("portScanner.scanning") : t("portScanner.scan")}
          </Button>
          {scanning && (
            <Button type="button" variant="outline" onClick={() => void invoke("cancel_port_scan")}>
              <Square />
              {t("portScanner.cancel")}
            </Button>
          )}
        </div>
      </form>

      {failure && <p className="text-sm text-destructive">{failure}</p>}

      {result && (
        <div className="flex flex-col gap-3">
          <p className="text-sm">
            {t("portScanner.summary", {
              host: result.host,
              address: result.address,
              scanned: result.scanned,
              open: result.open.length,
              seconds: (result.durationMs / 1000).toFixed(1),
            })}
            {result.cancelled && (
              <span className="ml-2 text-amber-600 dark:text-amber-400">{t("portScanner.cancelled")}</span>
            )}
          </p>
          {result.open.length === 0 ? (
            <p className="text-sm text-muted-foreground">{t("portScanner.noneOpen")}</p>
          ) : (
            <div className="overflow-x-auto rounded-lg border">
              <div className="min-w-[24rem]">
                <div className="grid grid-cols-[6rem_1fr] gap-2 border-b bg-muted/50 px-3 py-2 text-xs font-medium text-muted-foreground">
                  <span>{t("portScanner.columns.port")}</span>
                  <span>{t("portScanner.columns.service")}</span>
                </div>
                {result.open.map((p) => (
                  <div
                    key={p.port}
                    className="grid grid-cols-[6rem_1fr] items-center gap-2 border-b px-3 py-2 text-sm last:border-b-0"
                  >
                    <span className="font-mono">{p.port}</span>
                    <span className="text-muted-foreground">
                      {p.service ? <Badge variant="secondary">{p.service}</Badge> : t("common.none")}
                    </span>
                  </div>
                ))}
              </div>
            </div>
          )}
          <p className="text-xs text-muted-foreground">
            {t("portScanner.breakdown", {
              closed: result.closed,
              filtered: result.filtered,
              errors: result.errors,
            })}
          </p>
        </div>
      )}
    </div>
  );
}
