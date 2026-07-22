import type { ComponentType } from "react";
import {
  Database,
  FileBadge,
  FileCog,
  FileSignature,
  LayoutDashboard,
  Network,
  Radar,
  Route,
  Server,
  Timer,
  Waypoints,
} from "lucide-react";

export interface NavItem {
  id: NavId;
  labelKey: string;
  icon: ComponentType<{ className?: string }>;
}

export interface NavGroup {
  labelKey: string;
  items: NavItem[];
}

export type NavId =
  | "dashboard"
  | "nrpt-rules"
  | "connection-test"
  | "network-routes"
  | "dns-servers"
  | "dns-cache"
  | "dns-monitor"
  | "hosts-file"
  | "proxy-settings"
  | "code-signing"
  | "certificate-store";

export const navGroups: NavGroup[] = [
  {
    labelKey: "nav.overview",
    items: [
      { id: "dashboard", labelKey: "nav.dashboard", icon: LayoutDashboard },
    ],
  },
  {
    labelKey: "nav.network",
    items: [
      { id: "nrpt-rules", labelKey: "nav.nrptRules", icon: Network },
      { id: "connection-test", labelKey: "nav.connectionTest", icon: Radar },
      { id: "network-routes", labelKey: "nav.networkRoutes", icon: Route },
      { id: "dns-servers", labelKey: "nav.dnsServers", icon: Server },
      { id: "dns-cache", labelKey: "nav.dnsCache", icon: Database },
      { id: "dns-monitor", labelKey: "nav.dnsMonitor", icon: Timer },
      { id: "hosts-file", labelKey: "nav.hostsFile", icon: FileCog },
      { id: "proxy-settings", labelKey: "nav.proxySettings", icon: Waypoints },
    ],
  },
  {
    labelKey: "nav.devTools",
    items: [
      { id: "code-signing", labelKey: "nav.codeSigning", icon: FileSignature },
      {
        id: "certificate-store",
        labelKey: "nav.certificateStore",
        icon: FileBadge,
      },
    ],
  },
];
