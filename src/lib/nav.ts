import type { ComponentType } from "react";
import {
  Database,
  FileBadge,
  FileCog,
  FileSignature,
  History,
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
  BookOpen,
  Boxes,
  ClipboardList,
  LockKeyhole,
  Power,
  Rocket,
  ScanSearch,
  Variable,
  Languages,
  Cog,
  ScrollText,
  Share2,
  Shield,
  ShieldCheck,
  Wifi,
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
  | "manual"
  | "languages"
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
  | "firewall"
  | "neighbors"
  | "vpn"
  | "wifi"
  | "services"
  | "event-log"
  | "environment"
  | "wake-on-lan"
  | "port-scanner"
  | "startup"
  | "tls-inspector"
  | "diagnostics"
  | "snapshots"
  | "proxy-settings"
  | "code-signing"
  | "certificate-store";

export const navGroups: NavGroup[] = [
  {
    labelKey: "nav.overview",
    items: [
      { id: "dashboard", labelKey: "nav.dashboard", icon: LayoutDashboard },
      { id: "manual", labelKey: "nav.manual", icon: BookOpen },
      { id: "languages", labelKey: "nav.languages", icon: Languages },
    ],
  },
  {
    labelKey: "nav.network",
    items: [
      { id: "nrpt-rules", labelKey: "nav.nrptRules", icon: Network },
      { id: "connection-test", labelKey: "nav.connectionTest", icon: Radar },
      { id: "ports", labelKey: "nav.ports", icon: Plug },
      { id: "port-proxy", labelKey: "nav.portProxy", icon: ArrowRightLeft },
      { id: "port-scanner", labelKey: "nav.portScanner", icon: ScanSearch },
      { id: "wake-on-lan", labelKey: "nav.wakeOnLan", icon: Power },
      { id: "network-adapters", labelKey: "nav.adapters", icon: Cable },
      { id: "network-routes", labelKey: "nav.networkRoutes", icon: Route },
      { id: "dns-servers", labelKey: "nav.dnsServers", icon: Server },
      { id: "dns-lookup", labelKey: "nav.dnsLookup", icon: Globe },
      { id: "dns-cache", labelKey: "nav.dnsCache", icon: Database },
      { id: "dns-monitor", labelKey: "nav.dnsMonitor", icon: Timer },
      { id: "hosts-file", labelKey: "nav.hostsFile", icon: FileCog },
      { id: "ssh-config", labelKey: "nav.sshConfig", icon: Terminal },
      { id: "wsl", labelKey: "nav.wsl", icon: Boxes },
      { id: "firewall", labelKey: "nav.firewall", icon: Shield },
      { id: "neighbors", labelKey: "nav.neighbors", icon: Share2 },
      { id: "vpn", labelKey: "nav.vpn", icon: ShieldCheck },
      { id: "wifi", labelKey: "nav.wifi", icon: Wifi },
      { id: "proxy-settings", labelKey: "nav.proxySettings", icon: Waypoints },
    ],
  },
  {
    labelKey: "nav.system",
    items: [
      { id: "services", labelKey: "nav.services", icon: Cog },
      { id: "event-log", labelKey: "nav.eventLog", icon: ScrollText },
      { id: "environment", labelKey: "nav.environment", icon: Variable },
      { id: "startup", labelKey: "nav.startup", icon: Rocket },
      { id: "diagnostics", labelKey: "nav.diagnostics", icon: ClipboardList },
      { id: "snapshots", labelKey: "nav.snapshots", icon: History },
    ],
  },
  {
    labelKey: "nav.devTools",
    items: [
      { id: "code-signing", labelKey: "nav.codeSigning", icon: FileSignature },
      { id: "tls-inspector", labelKey: "nav.tlsInspector", icon: LockKeyhole },
      {
        id: "certificate-store",
        labelKey: "nav.certificateStore",
        icon: FileBadge,
      },
    ],
  },
];
