import { useState, type FormEvent } from "react";
import { invoke } from "@tauri-apps/api/core";
import { Loader2, Search } from "lucide-react";
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

const RECORD_TYPES = [
  "A",
  "AAAA",
  "CNAME",
  "MX",
  "NS",
  "TXT",
  "SOA",
  "PTR",
  "SRV",
  "CAA",
  "DNSKEY",
];

interface DnsRecord {
  name: string;
  recordType: string;
  ttl: number | null;
  section: string;
  data: string;
}

interface DnsLookupResult {
  records: DnsRecord[];
  error: string | null;
  queryTimeMs: number;
}

interface LastQuery {
  name: string;
  type: string;
  server: string | null;
}

const GRID = "grid-cols-[1.2fr_4.5rem_4.5rem_6rem_2fr]";

export function DnsLookupPage() {
  const { t } = useTranslation();
  const [name, setName] = useState("");
  const [type, setType] = useState("A");
  const [server, setServer] = useState("");
  const [running, setRunning] = useState(false);
  const [result, setResult] = useState<DnsLookupResult | null>(null);
  const [failure, setFailure] = useState<string | null>(null);
  const [last, setLast] = useState<LastQuery | null>(null);

  const canRun = name.trim().length > 0 && !running;

  async function lookup(e?: FormEvent) {
    e?.preventDefault();
    if (!canRun) return;
    setRunning(true);
    setFailure(null);
    const query = {
      name: name.trim(),
      type,
      server: server.trim() || null,
    };
    try {
      const res = await invoke<DnsLookupResult>("dns_lookup", {
        name: query.name,
        recordType: query.type,
        server: query.server,
      });
      setResult(res);
      setLast(query);
    } catch (err) {
      setResult(null);
      setFailure(err instanceof Error ? err.message : String(err));
    } finally {
      setRunning(false);
    }
  }

  return (
    <div className="flex flex-col gap-6">
      <div>
        <h1 className="text-lg font-semibold">{t("dnsLookup.title")}</h1>
        <p className="text-sm text-muted-foreground">{t("dnsLookup.subtitle")}</p>
      </div>

      <form onSubmit={lookup} className="flex flex-col gap-4">
        <div className="grid gap-4 sm:grid-cols-[2fr_8rem_1.5fr]">
          <div className="flex flex-col gap-1.5">
            <Label htmlFor="lookupName">{t("dnsLookup.nameLabel")}</Label>
            <Input
              id="lookupName"
              placeholder="example.com"
              value={name}
              onChange={(e) => setName(e.currentTarget.value)}
              autoFocus
            />
          </div>
          <div className="flex flex-col gap-1.5">
            <Label htmlFor="lookupType">{t("dnsLookup.typeLabel")}</Label>
            <Select value={type} onValueChange={(v) => setType(v as string)}>
              <SelectTrigger id="lookupType" className="w-full">
                <SelectValue>{(v: string) => v}</SelectValue>
              </SelectTrigger>
              <SelectContent>
                {RECORD_TYPES.map((r) => (
                  <SelectItem key={r} value={r}>
                    {r}
                  </SelectItem>
                ))}
              </SelectContent>
            </Select>
          </div>
          <div className="flex flex-col gap-1.5">
            <Label htmlFor="lookupServer">{t("dnsLookup.serverLabel")}</Label>
            <Input
              id="lookupServer"
              placeholder={t("dnsLookup.serverPlaceholder")}
              value={server}
              onChange={(e) => setServer(e.currentTarget.value)}
            />
          </div>
        </div>
        <div>
          <Button type="submit" disabled={!canRun}>
            {running ? <Loader2 className="animate-spin" /> : <Search />}
            {running ? t("dnsLookup.running") : t("dnsLookup.run")}
          </Button>
        </div>
      </form>

      {failure && <p className="text-sm text-destructive">{failure}</p>}

      {result && last && (
        <div className="flex flex-col gap-3">
          <p className="text-xs text-muted-foreground">
            {t("dnsLookup.summary", {
              type: last.type,
              name: last.name,
              server: last.server ?? t("dnsLookup.systemResolver"),
              ms: result.queryTimeMs,
            })}
          </p>
          {result.error && (
            <p className="text-sm text-destructive">{result.error}</p>
          )}
          {!result.error && result.records.length === 0 && (
            <p className="text-sm text-muted-foreground">{t("dnsLookup.noRecords")}</p>
          )}
          {result.records.length > 0 && (
            <div className="overflow-x-auto rounded-lg border">
              <div className="min-w-[40rem]">
                <div
                  className={`grid ${GRID} gap-2 border-b bg-muted/50 px-3 py-2 text-xs font-medium text-muted-foreground`}
                >
                  <span>{t("dnsLookup.columns.name")}</span>
                  <span>{t("dnsLookup.columns.type")}</span>
                  <span className="text-right">{t("dnsLookup.columns.ttl")}</span>
                  <span>{t("dnsLookup.columns.section")}</span>
                  <span>{t("dnsLookup.columns.data")}</span>
                </div>
                {result.records.map((r, i) => (
                  <div
                    key={`${r.name}|${r.recordType}|${r.data}|${i}`}
                    className={`grid ${GRID} items-start gap-2 border-b px-3 py-2 text-sm last:border-b-0`}
                  >
                    <span className="break-all">{r.name}</span>
                    <Badge variant="secondary" className="w-fit">
                      {r.recordType}
                    </Badge>
                    <span className="text-right text-muted-foreground">{r.ttl ?? ""}</span>
                    <span className="text-muted-foreground">{r.section}</span>
                    <span className="break-all font-mono text-xs">{r.data}</span>
                  </div>
                ))}
              </div>
            </div>
          )}
        </div>
      )}
    </div>
  );
}
