import { invoke } from "@tauri-apps/api/core";

export interface DnsResolveResult {
  hostname: string;
  server: string | null;
  resolved: boolean;
  addresses: string[];
  error: string | null;
  queryTimeMs: number;
}

export interface DnsMonitorEntry extends DnsResolveResult {
  id: string;
  timestamp: number;
}

export interface DnsMonitor {
  id: string;
  hostname: string;
  server: string;
  intervalSeconds: number;
  running: boolean;
  checking: boolean;
  entries: DnsMonitorEntry[];
}

const MAX_ENTRIES = 50;

function toErrorMessage(err: unknown): string {
  return err instanceof Error ? err.message : String(err);
}

// Module-level singleton, not component state: App only ever mounts the
// active page, so a monitor's polling loop has to live somewhere that
// survives navigating away from the DNS Monitor page. Every DnsMonitorPage
// instance subscribes to the same store instead of owning its own timers.
class DnsMonitorStore {
  private monitors: DnsMonitor[] = [];
  private listeners = new Set<() => void>();
  private timers = new Map<string, number>();

  subscribe = (listener: () => void): (() => void) => {
    this.listeners.add(listener);
    return () => {
      this.listeners.delete(listener);
    };
  };

  getSnapshot = (): DnsMonitor[] => this.monitors;

  private publish(next: DnsMonitor[]) {
    this.monitors = next;
    this.listeners.forEach((listener) => listener());
  }

  private patchMonitor(
    id: string,
    patch: Partial<DnsMonitor> | ((monitor: DnsMonitor) => Partial<DnsMonitor>),
  ) {
    this.publish(
      this.monitors.map((monitor) => {
        if (monitor.id !== id) return monitor;
        const resolved = typeof patch === "function" ? patch(monitor) : patch;
        return { ...monitor, ...resolved };
      }),
    );
  }

  addMonitor(hostname: string, server: string, intervalSeconds: number): string {
    const id = crypto.randomUUID();
    const monitor: DnsMonitor = {
      id,
      hostname,
      server,
      intervalSeconds,
      running: false,
      checking: false,
      entries: [],
    };
    this.publish([monitor, ...this.monitors]);
    this.start(id);
    return id;
  }

  removeMonitor(id: string) {
    this.clearTimer(id);
    this.publish(this.monitors.filter((monitor) => monitor.id !== id));
  }

  clearEntries(id: string) {
    this.patchMonitor(id, { entries: [] });
  }

  start(id: string) {
    const monitor = this.monitors.find((m) => m.id === id);
    if (!monitor || monitor.running) return;
    this.patchMonitor(id, { running: true });
    void this.runCheck(id).then(() => this.scheduleNext(id));
  }

  stop(id: string) {
    this.clearTimer(id);
    this.patchMonitor(id, { running: false, checking: false });
  }

  private clearTimer(id: string) {
    const timer = this.timers.get(id);
    if (timer !== undefined) {
      window.clearTimeout(timer);
      this.timers.delete(id);
    }
  }

  private scheduleNext(id: string) {
    this.clearTimer(id);
    const monitor = this.monitors.find((m) => m.id === id);
    if (!monitor || !monitor.running) return;

    const timer = window.setTimeout(async () => {
      const current = this.monitors.find((m) => m.id === id);
      if (!current || !current.running) return;
      await this.runCheck(id);
      this.scheduleNext(id);
    }, monitor.intervalSeconds * 1000);
    this.timers.set(id, timer);
  }

  private async runCheck(id: string) {
    const monitor = this.monitors.find((m) => m.id === id);
    if (!monitor) return;

    this.patchMonitor(id, { checking: true });
    try {
      const result = await invoke<DnsResolveResult>("resolve_hostname", {
        hostname: monitor.hostname,
        server: monitor.server || null,
      });
      this.appendEntry(id, {
        ...result,
        id: crypto.randomUUID(),
        timestamp: Date.now(),
      });
    } catch (err) {
      this.appendEntry(id, {
        hostname: monitor.hostname,
        server: monitor.server || null,
        resolved: false,
        addresses: [],
        error: toErrorMessage(err),
        queryTimeMs: 0,
        id: crypto.randomUUID(),
        timestamp: Date.now(),
      });
    } finally {
      this.patchMonitor(id, { checking: false });
    }
  }

  private appendEntry(id: string, entry: DnsMonitorEntry) {
    this.patchMonitor(id, (monitor) => ({
      entries: [entry, ...monitor.entries].slice(0, MAX_ENTRIES),
    }));
  }
}

export const dnsMonitorStore = new DnsMonitorStore();
