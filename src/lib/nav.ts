import type { ComponentType } from "react";
import {
  Database,
  FileBadge,
  FileCog,
  FileSignature,
  LayoutDashboard,
  Network,
  ArrowRightLeft,
  Cable,
  Globe,
  Plug,
  Radar,
  Route,
  Server,
  Terminal,
  Timer,
  Waypoints,
  Boxes,
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
  | "ports"
  | "port-proxy"
  | "network-adapters"
  | "dns-lookup"
  | "network-routes"
  | "dns-servers"
  | "dns-cache"
  | "dns-monitor"
  | "hosts-file"
  | "ssh-config"
  | "wsl"
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
      { id: "ports", labelKey: "nav.ports", icon: Plug },
      { id: "port-proxy", labelKey: "nav.portProxy", icon: ArrowRightLeft },
      { id: "network-adapters", labelKey: "nav.adapters", icon: Cable },
      { id: "network-routes", labelKey: "nav.networkRoutes", icon: Route },
      { id: "dns-servers", labelKey: "nav.dnsServers", icon: Server },
      { id: "dns-lookup", labelKey: "nav.dnsLookup", icon: Globe },
      { id: "dns-cache", labelKey: "nav.dnsCache", icon: Database },
      { id: "dns-monitor", labelKey: "nav.dnsMonitor", icon: Timer },
      { id: "hosts-file", labelKey: "nav.hostsFile", icon: FileCog },
      { id: "ssh-config", labelKey: "nav.sshConfig", icon: Terminal },
      { id: "wsl", labelKey: "nav.wsl", icon: Boxes },
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
