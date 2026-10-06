import { useState, type FormEvent } from "react";
import { invoke } from "@tauri-apps/api/core";
import { Loader2, LockKeyhole } from "lucide-react";
import { useTranslation } from "react-i18next";
import type { TFunction } from "i18next";

import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { DetailList, DetailRow } from "@/components/detail-list";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";

interface Certificate {
  subject: string;
  issuer: string;
  serial: string;
  thumbprint: string;
  notBefore: string;
  notAfter: string;
  daysRemaining: number;
  signature: string | null;
  publicKey: string | null;
  selfSigned: boolean;
  san: string[];
}

interface ChainElement {
  cert: Certificate;
  problems: string[];
}

interface Inspection {
  host: string;
  port: number;
  serverName: string;
  connected: boolean;
  handshake: boolean;
  protocol: string | null;
  cipher: string | null;
  cipherStrength: number | null;
  hash: string | null;
  keyExchange: string | null;
  policyErrors: string | null;
  trusted: boolean;
  nameMatches: boolean;
  certificate: Certificate | null;
  chain: ChainElement[];
  error: string | null;
  durationMs: number;
}

const EXPIRY_WARNING_DAYS = 30;

// Protocol versions that are deprecated and shouldn't be in use.
const WEAK_PROTOCOLS = /^(Ssl|Tls|Tls11)\b/;

function formatDate(iso: string): string {
  const date = new Date(iso);
  return Number.isNaN(date.getTime()) ? iso : date.toLocaleDateString();
}

function Verdict({ ok, label }: { ok: boolean; label: string }) {
  return <Badge variant={ok ? "default" : "destructive"}>{label}</Badge>;
}

function expiryText(days: number, t: TFunction) {
  if (days < 0) return t("tlsInspector.expiredDaysAgo", { count: -days });
  return t("tlsInspector.expiresInDays", { count: days });
}

function CertificateDetails({ cert, t }: { cert: Certificate; t: TFunction }) {
  return (
    <DetailList>
      <DetailRow label={t("tlsInspector.cert.subject")} value={cert.subject} />
      <DetailRow label={t("tlsInspector.cert.issuer")} value={cert.issuer} />
      <DetailRow label={t("tlsInspector.cert.validFrom")} value={formatDate(cert.notBefore)} />
      <DetailRow label={t("tlsInspector.cert.validUntil")} value={`${formatDate(cert.notAfter)} (${expiryText(cert.daysRemaining, t)})`} />
      <DetailRow label={t("tlsInspector.cert.publicKey")} value={cert.publicKey} />
      <DetailRow label={t("tlsInspector.cert.signature")} value={cert.signature} />
      <DetailRow label={t("tlsInspector.cert.serial")} value={cert.serial} />
      <DetailRow label={t("tlsInspector.cert.thumbprint")} value={cert.thumbprint} />
    </DetailList>
  );
}

function Report({ result, t }: { result: Inspection; t: TFunction }) {
  const cert = result.certificate;

  if (!result.connected) {
    return (
      <div className="rounded-lg border border-destructive/40 p-4 text-sm">
        <p className="font-medium text-destructive">{t("tlsInspector.couldntConnect")}</p>
        <p className="mt-1 text-muted-foreground">{result.error}</p>
      </div>
    );
  }
  if (!result.handshake && !cert) {
    return (
      <div className="rounded-lg border border-destructive/40 p-4 text-sm">
        <p className="font-medium text-destructive">{t("tlsInspector.noTls")}</p>
        <p className="mt-1 text-muted-foreground">{result.error}</p>
        <p className="mt-2 text-xs text-muted-foreground">{t("tlsInspector.noTlsHint")}</p>
      </div>
    );
  }

  const expired = cert ? cert.daysRemaining < 0 : false;
  const expiringSoon = cert ? cert.daysRemaining >= 0 && cert.daysRemaining <= EXPIRY_WARNING_DAYS : false;
  const weak = result.protocol ? WEAK_PROTOCOLS.test(result.protocol) : false;

  return (
    <div className="flex flex-col gap-5">
      <div className="flex flex-wrap items-center gap-2">
        <Verdict ok={result.trusted} label={result.trusted ? t("tlsInspector.trusted") : t("tlsInspector.notTrusted")} />
        <Verdict
          ok={result.nameMatches}
          label={result.nameMatches ? t("tlsInspector.nameMatches") : t("tlsInspector.nameMismatch", { name: result.serverName })}
        />
        {cert && (
          <Badge variant={expired ? "destructive" : expiringSoon ? "secondary" : "outline"}>
            {expired ? t("tlsInspector.expired") : expiryText(cert.daysRemaining, t)}
          </Badge>
        )}
        {weak && <Badge variant="destructive">{t("tlsInspector.weakProtocol", { protocol: result.protocol })}</Badge>}
      </div>
      {!result.trusted && <p className="text-sm text-muted-foreground">{t("tlsInspector.trustHint")}</p>}
      {!result.nameMatches && <p className="text-sm text-muted-foreground">{t("tlsInspector.nameHint")}</p>}
      {result.error && <p className="text-sm text-destructive">{result.error}</p>}

      <section className="flex flex-col gap-2">
        <h2 className="text-sm font-medium">{t("tlsInspector.connection")}</h2>
        <DetailList>
          <DetailRow label={t("tlsInspector.protocol")} value={result.protocol} />
          <DetailRow
            label={t("tlsInspector.cipher")}
            value={result.cipher ? `${result.cipher}${result.cipherStrength ? ` (${result.cipherStrength}-bit)` : ""}` : null}
          />
          <DetailRow label={t("tlsInspector.keyExchange")} value={result.keyExchange} />
          <DetailRow label={t("tlsInspector.hash")} value={result.hash} />
          <DetailRow label={t("tlsInspector.serverName")} value={result.serverName} />
          <DetailRow label={t("tlsInspector.time")} value={`${result.durationMs} ms`} />
        </DetailList>
      </section>

      {cert && (
        <section className="flex flex-col gap-2">
          <h2 className="text-sm font-medium">{t("tlsInspector.certificate")}</h2>
          <CertificateDetails cert={cert} t={t} />
          <div className="mt-1 flex flex-col gap-1">
            <span className="text-sm text-muted-foreground">{t("tlsInspector.cert.names")}</span>
            {cert.san.length === 0 ? (
              <span className="text-sm">{t("common.none")}</span>
            ) : (
              <div className="flex flex-wrap gap-1.5">
                {cert.san.map((name) => (
                  <Badge key={name} variant="outline" className="font-mono text-xs">
                    {name}
                  </Badge>
                ))}
              </div>
            )}
          </div>
        </section>
      )}

      {result.chain.length > 0 && (
        <section className="flex flex-col gap-2">
          <h2 className="text-sm font-medium">{t("tlsInspector.chain", { count: result.chain.length })}</h2>
          <div className="rounded-lg border">
            {result.chain.map((el, i) => (
              <div key={`${el.cert.thumbprint}|${i}`} className="border-b px-3 py-2 text-sm last:border-b-0">
                <div className="flex flex-wrap items-center gap-2">
                  <Badge variant="secondary">
                    {i === 0
                      ? t("tlsInspector.chainLeaf")
                      : i === result.chain.length - 1
                        ? t("tlsInspector.chainRoot")
                        : t("tlsInspector.chainIntermediate")}
                  </Badge>
                  <span className="font-medium break-all">{el.cert.subject}</span>
                </div>
                <p className="mt-0.5 text-xs text-muted-foreground">
                  {t("tlsInspector.issuedBy", { issuer: el.cert.issuer })} · {t("tlsInspector.cert.validUntil")}{" "}
                  {formatDate(el.cert.notAfter)}
                </p>
                {el.problems.map((p) => (
                  <p key={p} className="mt-1 text-xs text-destructive">
                    {p}
                  </p>
                ))}
              </div>
            ))}
          </div>
        </section>
      )}
    </div>
  );
}

export function TlsInspectorPage() {
  const { t } = useTranslation();
  const [host, setHost] = useState("");
  const [port, setPort] = useState("443");
  const [serverName, setServerName] = useState("");
  const [running, setRunning] = useState(false);
  const [result, setResult] = useState<Inspection | null>(null);
  const [failure, setFailure] = useState<string | null>(null);

  const portNumber = Number(port);
  const canRun = host.trim().length > 0 && Number.isInteger(portNumber) && portNumber >= 1 && portNumber <= 65535 && !running;

  async function inspect(e?: FormEvent) {
    e?.preventDefault();
    if (!canRun) return;
    setRunning(true);
    setFailure(null);
    try {
      setResult(
        await invoke<Inspection>("inspect_tls", {
          host,
          port: portNumber,
          serverName: serverName.trim() || null,
          timeoutMs: 10_000,
        }),
      );
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
        <h1 className="text-lg font-semibold">{t("tlsInspector.title")}</h1>
        <p className="text-sm text-muted-foreground">{t("tlsInspector.subtitle")}</p>
      </div>

      <form onSubmit={inspect} className="flex flex-col gap-4">
        <div className="grid gap-4 sm:grid-cols-[2fr_6rem_2fr]">
          <div className="flex flex-col gap-1.5">
            <Label htmlFor="tlsHost">{t("tlsInspector.hostLabel")}</Label>
            <Input
              id="tlsHost"
              value={host}
              onChange={(e) => setHost(e.currentTarget.value)}
              placeholder="example.com"
              disabled={running}
              autoFocus
            />
          </div>
          <div className="flex flex-col gap-1.5">
            <Label htmlFor="tlsPort">{t("tlsInspector.portLabel")}</Label>
            <Input
              id="tlsPort"
              value={port}
              onChange={(e) => setPort(e.currentTarget.value)}
              inputMode="numeric"
              disabled={running}
            />
          </div>
          <div className="flex flex-col gap-1.5">
            <Label htmlFor="tlsSni">{t("tlsInspector.sniLabel")}</Label>
            <Input
              id="tlsSni"
              value={serverName}
              onChange={(e) => setServerName(e.currentTarget.value)}
              placeholder={t("tlsInspector.sniPlaceholder")}
              disabled={running}
            />
          </div>
        </div>
        <div>
          <Button type="submit" disabled={!canRun}>
            {running ? <Loader2 className="animate-spin" /> : <LockKeyhole />}
            {running ? t("tlsInspector.running") : t("tlsInspector.inspect")}
          </Button>
        </div>
      </form>

      {failure && <p className="text-sm text-destructive">{failure}</p>}
      {result && <Report result={result} t={t} />}
    </div>
  );
}
