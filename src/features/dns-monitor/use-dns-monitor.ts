import { useSyncExternalStore } from "react";

import { dnsMonitorStore, type DnsMonitor } from "@/features/dns-monitor/dns-monitor-store";

export function useDnsMonitors(): DnsMonitor[] {
  return useSyncExternalStore(
    dnsMonitorStore.subscribe,
    dnsMonitorStore.getSnapshot,
  );
}

export { dnsMonitorStore };
export type { DnsMonitor, DnsMonitorEntry } from "@/features/dns-monitor/dns-monitor-store";
