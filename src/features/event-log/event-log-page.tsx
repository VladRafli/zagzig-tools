import { useCallback, useEffect, useRef, useState, type FormEvent } from "react";
import { invoke } from "@tauri-apps/api/core";
import { ChevronRight, Loader2, RefreshCw, Search } from "lucide-react";
import { useTranslation } from "react-i18next";
import type { TFunction } from "i18next";

import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";

interface EventEntry {
  time: string;
  level: number;
  provider: string;
  id: number;
  log: string;
  message: string;
}

interface EventLogResult {
  events: EventEntry[];
  error: string | null;
}

type Preset = "network" | "wsl-docker" | "system" | "application";
type Level = "errors" | "warnings" | "all";

const PRESETS: Preset[] = ["network", "wsl-docker", "system", "application"];
const LEVELS: Level[] = ["errors", "warnings", "all"];
const HOURS = [1, 6, 24, 168, 720];
const MAX_EVENTS = [100, 200, 500];

const GRID = "grid-cols-[1.25rem_10.5rem_5.5rem_1.4fr_4rem]";

function levelBadge(level: number, t: TFunction) {
  if (level <= 2) {
    return (
      <Badge variant="destructive" className="w-fit">
        {level === 1 ? t("eventLog.levels.critical") : t("eventLog.levels.error")}
      </Badge>
    );
  }
  if (level === 3) {
    return (
      <Badge variant="secondary" className="w-fit">
        {t("eventLog.levels.warning")}
      </Badge>
    );
  }
  return (
    <Badge variant="outline" className="w-fit">
      {t("eventLog.levels.info")}
    </Badge>
  );
}

function formatTime(iso: string): string {
  const date = new Date(iso);
  return Number.isNaN(date.getTime()) ? iso : date.toLocaleString();
}

function EventRow({ event, t }: { event: EventEntry; t: TFunction }) {
  const [open, setOpen] = useState(false);
  return (
    <div className="border-b last:border-b-0">
      <button
        type="button"
        onClick={() => setOpen((o) => !o)}
        aria-expanded={open}
        className={`grid ${GRID} w-full items-center gap-2 px-3 py-2 text-left text-sm hover:bg-muted/40`}
      >
        <ChevronRight
          className={`size-4 text-muted-foreground transition-transform ${open ? "rotate-90" : ""}`}
        />
        <span className="text-xs text-muted-foreground">{formatTime(event.time)}</span>
        {levelBadge(event.level, t)}
        <span className="truncate">
          <span className="font-medium">{event.provider}</span>
          <span className="ml-2 text-xs text-muted-foreground">
            {event.message.split(/\r?\n/)[0]}
          </span>
        </span>
        <span className="text-right text-xs text-muted-foreground">{event.id}</span>
      </button>
      {open && (
        <div className="border-t bg-muted/20 px-4 py-3">
          <p className="mb-2 text-xs text-muted-foreground">
            {t("eventLog.source", { log: event.log, id: event.id })}
          </p>
          <pre className="max-h-64 overflow-auto whitespace-pre-wrap break-words text-xs">
            {event.message || t("common.none")}
          </pre>
        </div>
      )}
    </div>
  );
}

export function EventLogPage() {
  const { t } = useTranslation();
  const [preset, setPreset] = useState<Preset>("network");
  const [level, setLevel] = useState<Level>("errors");
  const [hours, setHours] = useState(24);
  const [maxEvents, setMaxEvents] = useState(200);
  const [text, setText] = useState("");

  const [loading, setLoading] = useState(false);
  const [result, setResult] = useState<EventLogResult | null>(null);
  const [failure, setFailure] = useState<string | null>(null);
  const [loadedAt, setLoadedAt] = useState<Date | null>(null);
  // A slow query for an old filter must not overwrite a newer one.
  const requestId = useRef(0);

  const load = useCallback(async () => {
    const id = ++requestId.current;
    setLoading(true);
    setFailure(null);
    try {
      const res = await invoke<EventLogResult>("get_event_log", {
        presetId: preset,
        level,
        hours,
        maxEvents,
        text: text.trim() || null,
      });
      if (id !== requestId.current) return;
      setResult(res);
      setLoadedAt(new Date());
    } catch (err) {
      if (id !== requestId.current) return;
      setResult(null);
      setFailure(err instanceof Error ? err.message : String(err));
    } finally {
      if (id === requestId.current) setLoading(false);
    }
    // `text` is applied on submit, not on every keystroke.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [preset, level, hours, maxEvents]);

  useEffect(() => {
    void load();
  }, [load]);

  function submit(e: FormEvent) {
    e.preventDefault();
    void load();
  }

  const selectFor = <V extends string | number>(
    value: V,
    onChange: (v: V) => void,
    options: V[],
    labelFor: (v: V) => string,
    width: string,
    label: string,
  ) => (
    <Select value={String(value)} onValueChange={(v) => onChange((typeof value === "number" ? Number(v) : v) as V)}>
      <SelectTrigger size="sm" className={width} aria-label={label}>
        <SelectValue>{(v: string) => labelFor((typeof value === "number" ? Number(v) : v) as V)}</SelectValue>
      </SelectTrigger>
      <SelectContent>
        {options.map((o) => (
          <SelectItem key={String(o)} value={String(o)}>
            {labelFor(o)}
          </SelectItem>
        ))}
      </SelectContent>
    </Select>
  );

  const hoursLabel = (h: number) =>
    h < 24 ? t("eventLog.hours", { count: h }) : t("eventLog.days", { count: h / 24 });

  return (
    <div className="flex flex-col gap-6">
      <div>
        <h1 className="text-lg font-semibold">{t("eventLog.title")}</h1>
        <p className="text-sm text-muted-foreground">{t("eventLog.subtitle")}</p>
      </div>

      <form onSubmit={submit} className="flex flex-wrap items-center gap-2">
        {selectFor<Preset>(preset, setPreset, PRESETS, (v) => t(`eventLog.presets.${v}`), "w-44", t("eventLog.source"))}
        {selectFor<Level>(level, setLevel, LEVELS, (v) => t(`eventLog.levelFilters.${v}`), "w-40", t("eventLog.level"))}
        {selectFor<number>(hours, setHours, HOURS, hoursLabel, "w-36", t("eventLog.window"))}
        {selectFor<number>(maxEvents, setMaxEvents, MAX_EVENTS, (v) => t("eventLog.maxEvents", { count: v }), "w-32", t("eventLog.limit"))}
        <div className="relative min-w-40 flex-1">
          <Search className="pointer-events-none absolute top-1/2 left-2.5 size-4 -translate-y-1/2 text-muted-foreground" />
          <Input
            className="pl-8"
            placeholder={t("eventLog.searchPlaceholder")}
            value={text}
            onChange={(e) => setText(e.currentTarget.value)}
          />
        </div>
        <Button type="submit" variant="ghost" size="icon-sm" aria-label={t("eventLog.refresh")} disabled={loading}>
          <RefreshCw className={loading ? "animate-spin" : ""} />
        </Button>
      </form>

      {loading && !result && (
        <p className="flex items-center gap-2 text-sm text-muted-foreground">
          <Loader2 className="size-4 animate-spin" />
          {t("eventLog.reading")}
        </p>
      )}
      {failure && <p className="text-sm text-destructive">{t("eventLog.couldntRead", { error: failure })}</p>}
      {result?.error && <p className="text-sm text-destructive">{result.error}</p>}
      {result && !result.error && result.events.length === 0 && !loading && (
        <p className="text-sm text-muted-foreground">{t("eventLog.none")}</p>
      )}
      {result && result.events.length > 0 && (
        <div className="flex flex-col gap-2">
          <p className="text-xs text-muted-foreground">
            {t("eventLog.count", { count: result.events.length })}
            {loadedAt ? ` · ${loadedAt.toLocaleTimeString()}` : ""}
            {loading ? " …" : ""}
          </p>
          <div className="overflow-x-auto rounded-lg border">
            <div className="min-w-[44rem]">
              <div
                className={`grid ${GRID} gap-2 border-b bg-muted/50 px-3 py-2 text-xs font-medium text-muted-foreground`}
              >
                <span />
                <span>{t("eventLog.columns.time")}</span>
                <span>{t("eventLog.columns.level")}</span>
                <span>{t("eventLog.columns.event")}</span>
                <span className="text-right">{t("eventLog.columns.id")}</span>
              </div>
              {result.events.map((e, i) => (
                <EventRow key={`${e.time}|${e.provider}|${e.id}|${i}`} event={e} t={t} />
              ))}
            </div>
          </div>
        </div>
      )}
    </div>
  );
}
