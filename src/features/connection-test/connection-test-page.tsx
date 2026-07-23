import { useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { Loader2, Radar, Trash2 } from "lucide-react";
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
import { Console } from "@/components/console";
import { CollapsibleDetails } from "@/components/collapsible-details";
import {
  useConnectionHistory,
  type ConnectionHistoryEntry,
} from "@/features/connection-test/use-connection-history";
import { formatRelativeTime } from "@/lib/relative-time";

interface PingReply {
  success: boolean;
  status: string;
  roundtripTimeMs: number | null;
  address: string | null;
}

interface PingResult {
  target: string;
  replies: PingReply[];
}

interface TraceHop {
  hop: number;
  address: string | null;
  hostname: string | null;
  roundtripTimeMs: number | null;
  status: string;
}

// Mirrors tracert.exe's own "name [ip]" formatting for a resolved hop, since
// plenty of intermediate routers have a PTR record but no address alone
// means much to a person reading it.
function formatHopAddress(hop: TraceHop, t: TFunction): string {
  if (!hop.address) return t("connectionTest.path.noResponse");
  if (!hop.hostname) return hop.address;
  return `${hop.hostname} [${hop.address}]`;
}

interface TracerouteResult {
  target: string;
  hops: TraceHop[];
}

type RequestState<T> =
  | { status: "idle" }
  | { status: "loading" }
  | { status: "ready"; data: T }
  | { status: "error"; error: string };

function toErrorMessage(err: unknown): string {
  return err instanceof Error ? err.message : String(err);
}

function summarizePing(result: PingResult) {
  const { replies } = result;
  const successes = replies.filter((r) => r.success);
  const reachable = successes.length > 0;
  const lossPercent = Math.round(
    ((replies.length - successes.length) / replies.length) * 100,
  );
  const avgMs = successes.length
    ? Math.round(
        successes.reduce((sum, r) => sum + (r.roundtripTimeMs ?? 0), 0) /
          successes.length,
      )
    : null;

  return { reachable, lossPercent, avgMs, successCount: successes.length };
}

function ReachabilityCard({
  state,
  t,
}: {
  state: RequestState<PingResult>;
  t: TFunction;
}) {
  return (
    <Card>
      <CardHeader>
        <CardTitle>{t("connectionTest.reachability.title")}</CardTitle>
        <CardDescription>
          {t("connectionTest.reachability.description")}
        </CardDescription>
      </CardHeader>
      <CardContent>
        {state.status === "idle" && (
          <p className="text-sm text-muted-foreground">
            {t("connectionTest.reachability.runToSeeResults")}
          </p>
        )}
        {state.status === "loading" && (
          <p className="flex items-center gap-2 text-sm text-muted-foreground">
            <Loader2 className="size-4 animate-spin" />
            {t("connectionTest.reachability.checking")}
          </p>
        )}
        {state.status === "error" && (
          <p className="text-sm text-destructive">{state.error}</p>
        )}
        {state.status === "ready" && (
          <ReachabilitySummary result={state.data} t={t} />
        )}
      </CardContent>
    </Card>
  );
}

function ReachabilitySummary({
  result,
  t,
}: {
  result: PingResult;
  t: TFunction;
}) {
  const { reachable, lossPercent, avgMs, successCount } =
    summarizePing(result);
  const { replies } = result;

  return (
    <div className="flex flex-col gap-3">
      <div className="flex items-center gap-2">
        <Badge variant={reachable ? "default" : "destructive"}>
          {reachable
            ? t("connectionTest.reachability.reachable")
            : t("connectionTest.reachability.unreachable")}
        </Badge>
        <span className="text-sm text-muted-foreground">
          {avgMs !== null
            ? t("connectionTest.reachability.averageReplyTime", { ms: avgMs })
            : (replies[0]?.status ??
              t("connectionTest.reachability.noReplies"))}
        </span>
      </div>
      <p className="text-sm text-muted-foreground">
        {t("connectionTest.reachability.signalsAnswered", {
          count: successCount,
          total: replies.length,
          loss:
            lossPercent > 0
              ? t("connectionTest.reachability.lostSuffix", {
                  percent: lossPercent,
                })
              : "",
        })}
      </p>
    </div>
  );
}

function PathCard({
  state,
  t,
}: {
  state: RequestState<TracerouteResult>;
  t: TFunction;
}) {
  return (
    <Card>
      <CardHeader>
        <CardTitle>{t("connectionTest.path.title")}</CardTitle>
        <CardDescription>{t("connectionTest.path.description")}</CardDescription>
      </CardHeader>
      <CardContent>
        {state.status === "idle" && (
          <p className="text-sm text-muted-foreground">
            {t("connectionTest.path.runToSeeResults")}
          </p>
        )}
        {state.status === "loading" && (
          <p className="flex items-center gap-2 text-sm text-muted-foreground">
            <Loader2 className="size-4 animate-spin" />
            {t("connectionTest.path.tracing")}
          </p>
        )}
        {state.status === "error" && (
          <p className="text-sm text-destructive">{state.error}</p>
        )}
        {state.status === "ready" && (
          <HopList hops={state.data.hops} t={t} />
        )}
      </CardContent>
    </Card>
  );
}

function HopList({ hops, t }: { hops: TraceHop[]; t: TFunction }) {
  if (hops.length === 0) {
    return (
      <p className="text-sm text-muted-foreground">
        {t("connectionTest.path.noHops")}
      </p>
    );
  }

  return (
    <div className="overflow-hidden rounded-lg border">
      <div className="grid grid-cols-[3rem_1fr_5rem] gap-2 border-b bg-muted/50 px-3 py-2 text-xs font-medium text-muted-foreground">
        <span>{t("connectionTest.path.stop")}</span>
        <span>{t("connectionTest.path.address")}</span>
        <span className="text-right">{t("connectionTest.path.time")}</span>
      </div>
      {hops.map((hop) => (
        <div
          key={hop.hop}
          className="grid grid-cols-[3rem_1fr_5rem] gap-2 border-b px-3 py-2 text-sm last:border-b-0"
        >
          <span className="text-muted-foreground">{hop.hop}</span>
          <span className="break-all">{formatHopAddress(hop, t)}</span>
          <span className="text-right text-muted-foreground">
            {hop.roundtripTimeMs !== null
              ? `${hop.roundtripTimeMs} ms`
              : t("common.none")}
          </span>
        </div>
      ))}
    </div>
  );
}

function pad(value: string, width: number): string {
  return value.length >= width ? value : " ".repeat(width - value.length) + value;
}

function buildPingLines(
  t: TFunction,
  state: RequestState<PingResult>,
): string[] {
  if (state.status === "idle") return [];
  if (state.status === "loading")
    return [`> ${t("connectionTest.console.testingConnection")}`];
  if (state.status === "error")
    return [
      `> ${t("connectionTest.console.testingConnection")}`,
      t("connectionTest.console.error", { error: state.error }),
    ];

  const { target, replies } = state.data;
  const lines = [
    `> ${t("connectionTest.console.testingConnectionTo", { target })}`,
    "",
  ];
  replies.forEach((reply, i) => {
    if (reply.success) {
      lines.push(
        t("connectionTest.console.replyFrom", {
          address: reply.address ?? target,
          ms: reply.roundtripTimeMs,
        }),
      );
    } else {
      lines.push(
        t("connectionTest.console.requestFailed", {
          index: i + 1,
          status: reply.status,
        }),
      );
    }
  });
  const successes = replies.filter((r) => r.success).length;
  lines.push("");
  lines.push(
    t("connectionTest.console.sentReceivedLost", {
      sent: replies.length,
      received: successes,
      lost: replies.length - successes,
    }),
  );
  return lines;
}

function buildTraceLines(
  t: TFunction,
  state: RequestState<TracerouteResult>,
): string[] {
  if (state.status === "idle") return [];
  if (state.status === "loading")
    return [`> ${t("connectionTest.console.tracingPath")}`];
  if (state.status === "error")
    return [
      `> ${t("connectionTest.console.tracingPath")}`,
      t("connectionTest.console.error", { error: state.error }),
    ];

  const { target, hops } = state.data;
  const lines = [
    `> ${t("connectionTest.console.tracingRouteTo", { target })}`,
    "",
  ];
  hops.forEach((hop) => {
    const time = hop.roundtripTimeMs !== null ? `${hop.roundtripTimeMs}ms` : "*";
    const addr = hop.address
      ? formatHopAddress(hop, t)
      : t("connectionTest.console.requestTimedOut");
    lines.push(`${pad(String(hop.hop), 2)}  ${pad(time, 6)}  ${addr}`);
  });
  lines.push("");
  lines.push(t("connectionTest.console.traceComplete"));
  return lines;
}

function HistoryCard({
  entries,
  onClear,
  onRerun,
  t,
}: {
  entries: ConnectionHistoryEntry[];
  onClear: () => void;
  onRerun: (target: string) => void;
  t: TFunction;
}) {
  return (
    <Card>
      <CardHeader>
        <div className="flex items-center justify-between">
          <div>
            <CardTitle>{t("connectionTest.history.title")}</CardTitle>
            <CardDescription>
              {t("connectionTest.history.description")}
            </CardDescription>
          </div>
          {entries.length > 0 && (
            <Button variant="ghost" size="sm" onClick={onClear}>
              <Trash2 />
              {t("connectionTest.history.clear")}
            </Button>
          )}
        </div>
      </CardHeader>
      <CardContent>
        {entries.length === 0 ? (
          <p className="text-sm text-muted-foreground">
            {t("connectionTest.history.noTestsYet")}
          </p>
        ) : (
          <div className="flex flex-col gap-2">
            {entries.map((entry) => (
              <div
                key={entry.id}
                className="flex items-center justify-between gap-3 rounded-lg border px-3 py-2 text-sm"
              >
                <div className="flex flex-col">
                  <span className="font-medium">{entry.target}</span>
                  <span className="text-xs text-muted-foreground">
                    {formatRelativeTime(t, entry.timestamp)}
                    {entry.avgMs !== null
                      ? ` · ${t("connectionTest.history.avgMs", { ms: entry.avgMs })}`
                      : ""}
                    {entry.hopCount !== null
                      ? ` · ${t("connectionTest.history.hops", { count: entry.hopCount })}`
                      : ""}
                  </span>
                </div>
                <div className="flex items-center gap-2">
                  <Badge
                    variant={
                      entry.reachable === null
                        ? "secondary"
                        : entry.reachable
                          ? "default"
                          : "destructive"
                    }
                  >
                    {entry.reachable === null
                      ? t("connectionTest.history.error")
                      : entry.reachable
                        ? t("connectionTest.history.reachable")
                        : t("connectionTest.history.unreachable")}
                  </Badge>
                  <Button
                    variant="ghost"
                    size="sm"
                    onClick={() => onRerun(entry.target)}
                  >
                    {t("connectionTest.history.runAgain")}
                  </Button>
                </div>
              </div>
            ))}
          </div>
        )}
      </CardContent>
    </Card>
  );
}

export function ConnectionTestPage() {
  const { t } = useTranslation();
  const [target, setTarget] = useState("");
  const [pingState, setPingState] = useState<RequestState<PingResult>>({
    status: "idle",
  });
  const [traceState, setTraceState] = useState<RequestState<TracerouteResult>>(
    { status: "idle" },
  );
  const history = useConnectionHistory();

  const running = pingState.status === "loading" || traceState.status === "loading";
  const canRun = target.trim().length > 0 && !running;
  const hasOutput = pingState.status !== "idle" || traceState.status !== "idle";
  const outputLines = [
    ...buildPingLines(t, pingState),
    "",
    ...buildTraceLines(t, traceState),
  ];

  async function runTest(overrideHost?: string) {
    const host = (overrideHost ?? target).trim();
    if (!host || running) return;

    setTarget(host);
    setPingState({ status: "loading" });
    setTraceState({ status: "loading" });

    const [pingResult, traceResult] = await Promise.allSettled([
      invoke<PingResult>("ping_host", { target: host }),
      invoke<TracerouteResult>("traceroute_host", { target: host }),
    ]);

    let reachable: boolean | null = null;
    let avgMs: number | null = null;
    let lossPercent: number | null = null;
    let pingError: string | null = null;

    if (pingResult.status === "fulfilled") {
      setPingState({ status: "ready", data: pingResult.value });
      const summary = summarizePing(pingResult.value);
      reachable = summary.reachable;
      avgMs = summary.avgMs;
      lossPercent = summary.lossPercent;
    } else {
      pingError = toErrorMessage(pingResult.reason);
      setPingState({ status: "error", error: pingError });
    }

    let hopCount: number | null = null;
    let traceError: string | null = null;

    if (traceResult.status === "fulfilled") {
      setTraceState({ status: "ready", data: traceResult.value });
      hopCount = traceResult.value.hops.length;
    } else {
      traceError = toErrorMessage(traceResult.reason);
      setTraceState({ status: "error", error: traceError });
    }

    history.addEntry({
      target: host,
      reachable,
      avgMs,
      lossPercent,
      hopCount,
      pingError,
      traceError,
    });
  }

  return (
    <div className="flex flex-col gap-6">
      <div>
        <h1 className="text-lg font-semibold">{t("connectionTest.title")}</h1>
        <p className="text-sm text-muted-foreground">
          {t("connectionTest.subtitle")}
        </p>
      </div>

      <Card>
        <CardContent className="flex flex-col gap-4">
          <div className="flex flex-col gap-1.5">
            <Label htmlFor="target">{t("connectionTest.addressLabel")}</Label>
            <Input
              id="target"
              placeholder={t("connectionTest.addressPlaceholder")}
              value={target}
              onChange={(e) => setTarget(e.currentTarget.value)}
              onKeyDown={(e) => {
                if (e.key === "Enter") runTest();
              }}
            />
          </div>
          <div>
            <Button onClick={() => runTest()} disabled={!canRun}>
              {running ? (
                <Loader2 className="animate-spin" />
              ) : (
                <Radar />
              )}
              {t("connectionTest.runTest")}
            </Button>
          </div>
        </CardContent>
      </Card>

      <HistoryCard
        entries={history.entries}
        onClear={history.clear}
        onRerun={(host) => void runTest(host)}
        t={t}
      />

      <div className="grid gap-4 lg:grid-cols-2">
        <ReachabilityCard state={pingState} t={t} />
        <PathCard state={traceState} t={t} />
      </div>

      {hasOutput && (
        <Card>
          <CardContent>
            <CollapsibleDetails label={t("connectionTest.commandOutput")}>
              <Console lines={outputLines} className="mt-3" />
            </CollapsibleDetails>
          </CardContent>
        </Card>
      )}
    </div>
  );
}
