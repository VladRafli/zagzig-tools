import { useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";
import {
  CheckCircle2,
  FolderOpen,
  Loader2,
  RefreshCw,
  ShieldQuestion,
} from "lucide-react";
import { useTranslation } from "react-i18next";
import type { TFunction } from "i18next";

import { Button } from "@/components/ui/button";
import {
  Card,
  CardHeader,
  CardTitle,
  CardDescription,
  CardContent,
} from "@/components/ui/card";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Badge } from "@/components/ui/badge";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { Tabs, TabsList, TabsTrigger, TabsContent } from "@/components/ui/tabs";
import { Console } from "@/components/console";
import { useSigntool } from "@/features/code-signing/use-signtool";
import { useCodeSigningCertificates } from "@/features/code-signing/use-code-signing-certificates";

interface SigntoolOutput {
  success: boolean;
  output: string;
}

type RunState =
  | { status: "idle" }
  | { status: "running" }
  | { status: "done"; result: SigntoolOutput }
  | { status: "error"; error: string };

function toErrorMessage(err: unknown): string {
  return err instanceof Error ? err.message : String(err);
}

function extractCommonName(subject: string): string {
  const match = subject
    .split(",")
    .map((part) => part.trim())
    .find((part) => part.startsWith("CN="));
  return match ? match.slice(3) : subject;
}

function SigntoolStatusCard({
  t,
  signtool,
}: {
  t: TFunction;
  signtool: ReturnType<typeof useSigntool>;
}) {
  return (
    <Card>
      <CardHeader>
        <CardTitle>{t("codeSigning.signtool.title")}</CardTitle>
        <CardDescription>{t("codeSigning.signtool.description")}</CardDescription>
      </CardHeader>
      <CardContent>
        {signtool.status === "loading" && (
          <p className="flex items-center gap-2 text-sm text-muted-foreground">
            <Loader2 className="size-4 animate-spin" />
            {t("codeSigning.signtool.looking")}
          </p>
        )}
        {signtool.status === "ready" && (
          <div className="flex flex-col gap-3">
            <div className="flex flex-wrap items-center gap-2">
              <Badge variant={signtool.found ? "default" : "destructive"}>
                {signtool.found
                  ? t("codeSigning.signtool.found")
                  : t("codeSigning.signtool.notFound")}
              </Badge>
              {signtool.found && signtool.path && (
                <span className="break-all font-mono text-xs text-muted-foreground">
                  {signtool.path}
                </span>
              )}
            </div>
            {!signtool.found && (
              <p className="text-sm text-muted-foreground">
                {t("codeSigning.signtool.notFoundDescription")}
              </p>
            )}
            <div className="flex items-center gap-2">
              <Button variant="outline" size="sm" onClick={signtool.locate}>
                <FolderOpen />
                {t("codeSigning.signtool.locate")}
              </Button>
              {signtool.isOverride && (
                <Button
                  variant="ghost"
                  size="sm"
                  onClick={signtool.clearOverride}
                >
                  <RefreshCw />
                  {t("codeSigning.signtool.autoDetect")}
                </Button>
              )}
            </div>
          </div>
        )}
      </CardContent>
    </Card>
  );
}

async function browseForFile(title: string): Promise<string | null> {
  const selected = await open({ title, multiple: false });
  return typeof selected === "string" ? selected : null;
}

function FilePickerField({
  id,
  label,
  value,
  placeholder,
  dialogTitle,
  disabled,
  onChange,
  t,
}: {
  id: string;
  label: string;
  value: string;
  placeholder: string;
  dialogTitle: string;
  disabled?: boolean;
  onChange: (value: string) => void;
  t: TFunction;
}) {
  return (
    <div className="flex flex-col gap-1.5">
      <Label htmlFor={id}>{label}</Label>
      <div className="flex items-center gap-2">
        <Input
          id={id}
          value={value}
          placeholder={placeholder}
          disabled={disabled}
          onChange={(e) => onChange(e.currentTarget.value)}
        />
        <Button
          type="button"
          variant="outline"
          disabled={disabled}
          onClick={async () => {
            const picked = await browseForFile(dialogTitle);
            if (picked) onChange(picked);
          }}
        >
          <FolderOpen />
          {t("codeSigning.browse")}
        </Button>
      </div>
    </div>
  );
}

function ResultConsole({
  state,
  t,
}: {
  state: RunState;
  t: TFunction;
}) {
  if (state.status === "idle") return null;

  if (state.status === "running") {
    return (
      <p className="flex items-center gap-2 text-sm text-muted-foreground">
        <Loader2 className="size-4 animate-spin" />
        {t("codeSigning.running")}
      </p>
    );
  }

  if (state.status === "error") {
    return <p className="text-sm text-destructive">{state.error}</p>;
  }

  const lines = state.result.output.length
    ? state.result.output.split("\n")
    : [];

  return (
    <div className="flex flex-col gap-2">
      <Badge variant={state.result.success ? "default" : "destructive"}>
        {state.result.success
          ? t("codeSigning.succeeded")
          : t("codeSigning.failed")}
      </Badge>
      <Console lines={lines} />
    </div>
  );
}

function SignFileCard({
  t,
  signtoolFound,
  signtoolPath,
}: {
  t: TFunction;
  signtoolFound: boolean;
  signtoolPath: string | null;
}) {
  const certificates = useCodeSigningCertificates();
  const [filePath, setFilePath] = useState("");
  const [source, setSource] = useState<"store" | "pfx">("store");
  const [thumbprint, setThumbprint] = useState("");
  const [pfxPath, setPfxPath] = useState("");
  const [pfxPassword, setPfxPassword] = useState("");
  const [digestAlgorithm, setDigestAlgorithm] = useState("SHA256");
  const [timestampUrl, setTimestampUrl] = useState("");
  const [description, setDescription] = useState("");
  const [state, setState] = useState<RunState>({ status: "idle" });

  const disabled = !signtoolFound;
  const canSign =
    signtoolFound &&
    filePath.trim().length > 0 &&
    (source === "store" ? thumbprint.length > 0 : pfxPath.trim().length > 0) &&
    state.status !== "running";

  async function sign() {
    if (!canSign) return;
    setState({ status: "running" });
    try {
      const result = await invoke<SigntoolOutput>("sign_file", {
        signtoolPath: signtoolPath ?? "",
        filePath: filePath.trim(),
        thumbprint: source === "store" ? thumbprint : undefined,
        pfxPath: source === "pfx" ? pfxPath.trim() : undefined,
        pfxPassword: source === "pfx" ? pfxPassword : undefined,
        digestAlgorithm,
        timestampUrl: timestampUrl.trim() || undefined,
        description: description.trim() || undefined,
      });
      setState({ status: "done", result });
    } catch (err) {
      setState({ status: "error", error: toErrorMessage(err) });
    }
  }

  return (
    <Card>
      <CardHeader>
        <CardTitle>{t("codeSigning.sign.title")}</CardTitle>
        <CardDescription>{t("codeSigning.sign.description")}</CardDescription>
      </CardHeader>
      <CardContent className="flex flex-col gap-4">
        <FilePickerField
          id="signFile"
          label={t("codeSigning.sign.fileLabel")}
          value={filePath}
          placeholder={t("codeSigning.sign.filePlaceholder")}
          dialogTitle={t("codeSigning.sign.fileLabel")}
          disabled={disabled}
          onChange={setFilePath}
          t={t}
        />

        <Tabs value={source} onValueChange={(v) => setSource(v as "store" | "pfx")}>
          <TabsList>
            <TabsTrigger value="store" disabled={disabled}>
              {t("codeSigning.sign.sourceStore")}
            </TabsTrigger>
            <TabsTrigger value="pfx" disabled={disabled}>
              {t("codeSigning.sign.sourcePfx")}
            </TabsTrigger>
          </TabsList>
          <TabsContent value="store" className="flex flex-col gap-2 pt-2">
            {certificates.status === "loading" && (
              <p className="flex items-center gap-2 text-sm text-muted-foreground">
                <Loader2 className="size-4 animate-spin" />
                {t("codeSigning.sign.readingCertificates")}
              </p>
            )}
            {certificates.status === "error" && (
              <p className="text-sm text-destructive">
                {t("codeSigning.sign.couldntReadCertificates", {
                  error: certificates.error,
                })}
              </p>
            )}
            {certificates.status === "ready" && certificates.certificates.length === 0 && (
              <p className="text-sm text-muted-foreground">
                {t("codeSigning.sign.noCertificates")}
              </p>
            )}
            {certificates.certificates.length > 0 && (
              <Select
                value={thumbprint}
                onValueChange={(value) => setThumbprint((value as string) ?? "")}
                disabled={disabled}
              >
                <SelectTrigger className="w-full">
                  <SelectValue
                    placeholder={t("codeSigning.sign.certificatePlaceholder")}
                  />
                </SelectTrigger>
                <SelectContent>
                  {certificates.certificates.map((cert) => (
                    <SelectItem key={cert.thumbprint} value={cert.thumbprint}>
                      {extractCommonName(cert.subject)} —{" "}
                      {t("codeSigning.sign.expires", { date: cert.notAfter })}
                    </SelectItem>
                  ))}
                </SelectContent>
              </Select>
            )}
            <div className="flex items-center justify-end">
              <Button
                variant="ghost"
                size="sm"
                onClick={certificates.refresh}
                disabled={certificates.status === "loading"}
              >
                <RefreshCw
                  className={
                    certificates.status === "loading" ? "animate-spin" : ""
                  }
                />
                {t("codeSigning.sign.refreshCertificates")}
              </Button>
            </div>
          </TabsContent>
          <TabsContent value="pfx" className="flex flex-col gap-3 pt-2">
            <FilePickerField
              id="pfxFile"
              label={t("codeSigning.sign.pfxFileLabel")}
              value={pfxPath}
              placeholder={t("codeSigning.sign.pfxFilePlaceholder")}
              dialogTitle={t("codeSigning.sign.pfxFileLabel")}
              disabled={disabled}
              onChange={setPfxPath}
              t={t}
            />
            <div className="flex flex-col gap-1.5">
              <Label htmlFor="pfxPassword">
                {t("codeSigning.sign.pfxPasswordLabel")}
              </Label>
              <Input
                id="pfxPassword"
                type="password"
                value={pfxPassword}
                placeholder={t("codeSigning.sign.pfxPasswordPlaceholder")}
                disabled={disabled}
                onChange={(e) => setPfxPassword(e.currentTarget.value)}
              />
            </div>
          </TabsContent>
        </Tabs>

        <div className="grid gap-4 sm:grid-cols-2">
          <div className="flex flex-col gap-1.5">
            <Label>{t("codeSigning.sign.digestLabel")}</Label>
            <Select
              value={digestAlgorithm}
              onValueChange={(value) => setDigestAlgorithm((value as string) ?? "SHA256")}
              disabled={disabled}
            >
              <SelectTrigger className="w-full">
                <SelectValue />
              </SelectTrigger>
              <SelectContent>
                <SelectItem value="SHA256">SHA256</SelectItem>
                <SelectItem value="SHA1">SHA1</SelectItem>
              </SelectContent>
            </Select>
          </div>
          <div className="flex flex-col gap-1.5">
            <Label htmlFor="timestampUrl">
              {t("codeSigning.sign.timestampLabel")}
            </Label>
            <Input
              id="timestampUrl"
              value={timestampUrl}
              placeholder={t("codeSigning.sign.timestampPlaceholder")}
              disabled={disabled}
              onChange={(e) => setTimestampUrl(e.currentTarget.value)}
            />
          </div>
        </div>

        <div className="flex flex-col gap-1.5">
          <Label htmlFor="signDescription">
            {t("codeSigning.sign.descriptionLabel")}
          </Label>
          <Input
            id="signDescription"
            value={description}
            placeholder={t("codeSigning.sign.descriptionPlaceholder")}
            disabled={disabled}
            onChange={(e) => setDescription(e.currentTarget.value)}
          />
        </div>

        <div>
          <Button onClick={sign} disabled={!canSign}>
            {state.status === "running" ? (
              <Loader2 className="animate-spin" />
            ) : (
              <CheckCircle2 />
            )}
            {t("codeSigning.sign.action")}
          </Button>
        </div>

        <ResultConsole state={state} t={t} />
      </CardContent>
    </Card>
  );
}

function VerifyFileCard({
  t,
  signtoolFound,
  signtoolPath,
}: {
  t: TFunction;
  signtoolFound: boolean;
  signtoolPath: string | null;
}) {
  const [filePath, setFilePath] = useState("");
  const [state, setState] = useState<RunState>({ status: "idle" });

  const canVerify = signtoolFound && filePath.trim().length > 0 && state.status !== "running";

  async function verify() {
    if (!canVerify) return;
    setState({ status: "running" });
    try {
      const result = await invoke<SigntoolOutput>("verify_file", {
        signtoolPath: signtoolPath ?? "",
        filePath: filePath.trim(),
      });
      setState({ status: "done", result });
    } catch (err) {
      setState({ status: "error", error: toErrorMessage(err) });
    }
  }

  return (
    <Card>
      <CardHeader>
        <CardTitle>{t("codeSigning.verify.title")}</CardTitle>
        <CardDescription>{t("codeSigning.verify.description")}</CardDescription>
      </CardHeader>
      <CardContent className="flex flex-col gap-4">
        <FilePickerField
          id="verifyFile"
          label={t("codeSigning.verify.fileLabel")}
          value={filePath}
          placeholder={t("codeSigning.verify.filePlaceholder")}
          dialogTitle={t("codeSigning.verify.fileLabel")}
          disabled={!signtoolFound}
          onChange={setFilePath}
          t={t}
        />

        <div>
          <Button onClick={verify} disabled={!canVerify}>
            {state.status === "running" ? (
              <Loader2 className="animate-spin" />
            ) : (
              <ShieldQuestion />
            )}
            {t("codeSigning.verify.action")}
          </Button>
        </div>

        <ResultConsole state={state} t={t} />
      </CardContent>
    </Card>
  );
}

export function CodeSigningPage() {
  const { t } = useTranslation();
  const signtool = useSigntool();

  return (
    <div className="flex flex-col gap-6">
      <div>
        <h1 className="text-lg font-semibold">{t("codeSigning.title")}</h1>
        <p className="text-sm text-muted-foreground">{t("codeSigning.subtitle")}</p>
      </div>

      <SigntoolStatusCard t={t} signtool={signtool} />

      <div className="grid gap-4 lg:grid-cols-2">
        <SignFileCard
          t={t}
          signtoolFound={signtool.found}
          signtoolPath={signtool.path}
        />
        <VerifyFileCard
          t={t}
          signtoolFound={signtool.found}
          signtoolPath={signtool.path}
        />
      </div>
    </div>
  );
}
