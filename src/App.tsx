import { Suspense, lazy, useEffect, useState } from "react";
import { ArrowLeft, Search, Star } from "lucide-react";
import { useTranslation } from "react-i18next";

import { Button } from "@/components/ui/button";

import {
  Sidebar,
  SidebarContent,
  SidebarFooter,
  SidebarGroup,
  SidebarGroupContent,
  SidebarGroupLabel,
  SidebarHeader,
  SidebarInset,
  SidebarMenu,
  SidebarMenuAction,
  SidebarMenuButton,
  SidebarMenuItem,
  SidebarProvider,
  SidebarTrigger,
} from "@/components/ui/sidebar";
import { CommandPalette } from "@/components/command-palette";
import { LanguageSwitcher } from "@/components/language-switcher";
import { ThemeSwitcher } from "@/components/theme-switcher";
import { AdminStatusBadge } from "@/components/admin-status-badge";
import { AppVersion } from "@/components/app-version";
import { TitleBar } from "@/components/title-bar";
import { UpdateButton } from "@/components/update-button";
import { CertificatesPage } from "@/features/certificates/certificates-page";
import { CodeSigningPage } from "@/features/code-signing/code-signing-page";
import { ConnectionTestPage } from "@/features/connection-test/connection-test-page";
import { DashboardPage } from "@/features/dashboard/dashboard-page";
import { DnsPage } from "@/features/dns/dns-page";
import { DnsCachePage } from "@/features/dns-cache/dns-cache-page";
import { DnsMonitorPage } from "@/features/dns-monitor/dns-monitor-page";
import { HostsPage } from "@/features/hosts/hosts-page";
import { SshPage } from "@/features/ssh/ssh-page";
import { WslPage } from "@/features/wsl/wsl-page";
import { PortsPage } from "@/features/ports/ports-page";
import { PortProxyPage } from "@/features/port-proxy/port-proxy-page";
import { AdaptersPage } from "@/features/adapters/adapters-page";
import { DnsLookupPage } from "@/features/dns-lookup/dns-lookup-page";
import { FirewallPage } from "@/features/firewall/firewall-page";
import { NeighborsPage } from "@/features/neighbors/neighbors-page";
import { VpnPage } from "@/features/vpn/vpn-page";
import { WifiPage } from "@/features/wifi/wifi-page";
import { ServicesPage } from "@/features/services/services-page";
import { EventLogPage } from "@/features/event-log/event-log-page";
import { LanguagesPage } from "@/features/languages/languages-page";
import { EnvironmentPage } from "@/features/environment/environment-page";
import { WakeOnLanPage } from "@/features/wake-on-lan/wake-on-lan-page";
import { PortScannerPage } from "@/features/port-scanner/port-scanner-page";
import { StartupPage } from "@/features/startup/startup-page";
import { TlsInspectorPage } from "@/features/tls-inspector/tls-inspector-page";
import { DiagnosticsPage } from "@/features/diagnostics/diagnostics-page";
import { SnapshotsPage } from "@/features/snapshots/snapshots-page";

// The manual pulls in a Markdown renderer, so it loads only when opened.
const ManualPage = lazy(() =>
  import("@/features/manual/manual-page").then((m) => ({ default: m.ManualPage })),
);
import { NrptRulesPage } from "@/features/nrpt/nrpt-rules-page";
import { ProxyPage } from "@/features/proxy/proxy-page";
import { RoutingPage } from "@/features/routing/routing-page";
import { navGroups, type NavId, type NavItem } from "@/lib/nav";
import { toggleStarred, useStarred } from "@/lib/use-starred";

function App() {
  const { t } = useTranslation();
  const [active, setActive] = useState<NavId>("dashboard");
  // The dashboard already lists every feature, so the sidebar starts hidden
  // there and opens when a feature is picked. The user can still toggle it.
  const starred = useStarred();
  const starredItems = starred
    .map((id) => navGroups.flatMap((g) => g.items).find((i) => i.id === id))
    .filter((i): i is NavItem => !!i);
  const [sidebarOpen, setSidebarOpen] = useState(false);
  const [paletteOpen, setPaletteOpen] = useState(false);

  useEffect(() => {
    const onKeyDown = (event: KeyboardEvent) => {
      if ((event.ctrlKey || event.metaKey) && event.key.toLowerCase() === "k") {
        event.preventDefault();
        setPaletteOpen((open) => !open);
      }
    };
    window.addEventListener("keydown", onKeyDown);
    return () => window.removeEventListener("keydown", onKeyDown);
  }, []);

  const navigate = (id: NavId) => {
    setActive(id);
    setSidebarOpen(id !== "dashboard");
  };

  const renderItem = (item: NavItem, keyPrefix = "") => {
    const isStarred = starred.includes(item.id);
    const label = t(item.labelKey);
    const starLabel = t(isStarred ? "dashboard.unstar" : "dashboard.star", { name: label });
    return (
      <SidebarMenuItem key={keyPrefix + item.id}>
        <SidebarMenuButton
          isActive={active === item.id}
          onClick={() => navigate(item.id)}
        >
          <item.icon />
          <span>{label}</span>
        </SidebarMenuButton>
        {item.id !== "dashboard" && (
          <SidebarMenuAction
            showOnHover={!isStarred}
            aria-label={starLabel}
            title={starLabel}
            onClick={() => toggleStarred(item.id)}
          >
            <Star className={isStarred ? "fill-current text-yellow-500" : ""} />
          </SidebarMenuAction>
        )}
      </SidebarMenuItem>
    );
  };

  return (
    <div className="flex h-screen flex-col overflow-hidden">
      <TitleBar />
      {/* [contain:layout] makes this the containing block for Sidebar's
          `fixed inset-y-0` panel, so it's positioned relative to the space
          below the title bar instead of the real viewport (which would put
          it behind the title bar). */}
      <SidebarProvider
        open={sidebarOpen}
        onOpenChange={setSidebarOpen}
        className="min-h-0 flex-1 [contain:layout]"
      >
        {/* h-full overrides the library's default h-svh — svh is a
            viewport-relative unit that [contain:layout] on the parent can't
            touch (it only rescopes *positioning*, not *sizing*), so the
            fixed sidebar panel was sized to the whole window and overflowed
            past the bottom by exactly the title bar's height, clipping the
            footer. h-full resolves against the [contain:layout] box instead. */}
        <Sidebar className="h-full">
          <SidebarHeader>
            <span className="px-2 text-sm font-semibold">{t("app.title")}</span>
          </SidebarHeader>
          <SidebarContent>
            {starredItems.length > 0 && (
              <SidebarGroup>
                <SidebarGroupLabel>{t("dashboard.starred")}</SidebarGroupLabel>
                <SidebarGroupContent>
                  <SidebarMenu>
                    {starredItems.map((item) => renderItem(item, "starred-"))}
                  </SidebarMenu>
                </SidebarGroupContent>
              </SidebarGroup>
            )}
            {navGroups.map((group) => (
              <SidebarGroup key={group.labelKey}>
                <SidebarGroupLabel>{t(group.labelKey)}</SidebarGroupLabel>
                <SidebarGroupContent>
                  <SidebarMenu>
                    {group.items.map((item) => renderItem(item))}
                  </SidebarMenu>
                </SidebarGroupContent>
              </SidebarGroup>
            ))}
          </SidebarContent>
          <SidebarFooter className="gap-2">
            <UpdateButton />
            <ThemeSwitcher />
            <LanguageSwitcher />
            <AppVersion />
            <a
              href="https://github.com/VladRafli"
              target="_blank"
              rel="noreferrer"
              className="px-2 text-xs text-muted-foreground hover:text-foreground hover:underline"
            >
              {t("app.madeBy")}
            </a>
          </SidebarFooter>
        </Sidebar>
        <SidebarInset className="min-h-0">
          <header className="flex h-12 shrink-0 items-center gap-2 border-b px-4">
            <SidebarTrigger />
            {active !== "dashboard" && (
              <Button
                variant="ghost"
                size="sm"
                onClick={() => navigate("dashboard")}
              >
                <ArrowLeft />
                {t("nav.backToDashboard")}
              </Button>
            )}
            <div className="ml-auto flex items-center gap-2">
              <Button
                variant="outline"
                size="sm"
                onClick={() => setPaletteOpen(true)}
                aria-label={t("palette.open")}
                title={t("palette.open")}
                className="text-muted-foreground"
              >
                <Search />
                <span className="hidden sm:inline">{t("palette.open")}</span>
                <kbd className="rounded border px-1 text-[10px] font-normal">Ctrl K</kbd>
              </Button>
              <AdminStatusBadge />
            </div>
          </header>
          <main className="min-h-0 flex-1 overflow-y-auto p-6">
            {active === "dashboard" && <DashboardPage onNavigate={navigate} />}
            {active === "languages" && <LanguagesPage />}
            {active === "manual" && (
              <Suspense fallback={null}>
                <ManualPage />
              </Suspense>
            )}
            {active === "nrpt-rules" && <NrptRulesPage />}
            {active === "connection-test" && <ConnectionTestPage />}
            {active === "ports" && <PortsPage />}
            {active === "port-proxy" && <PortProxyPage />}
            {active === "network-adapters" && <AdaptersPage />}
            {active === "dns-lookup" && <DnsLookupPage />}
            {active === "firewall" && <FirewallPage />}
            {active === "neighbors" && <NeighborsPage />}
            {active === "vpn" && <VpnPage />}
            {active === "wifi" && <WifiPage />}
            {active === "services" && <ServicesPage />}
            {active === "event-log" && <EventLogPage />}
            {active === "environment" && <EnvironmentPage />}
            {active === "wake-on-lan" && <WakeOnLanPage />}
            {active === "port-scanner" && <PortScannerPage />}
            {active === "startup" && <StartupPage />}
            {active === "tls-inspector" && <TlsInspectorPage />}
            {active === "diagnostics" && <DiagnosticsPage />}
            {active === "snapshots" && <SnapshotsPage />}
            {active === "network-routes" && <RoutingPage />}
            {active === "dns-servers" && <DnsPage />}
            {active === "dns-cache" && <DnsCachePage />}
            {active === "dns-monitor" && <DnsMonitorPage />}
            {active === "hosts-file" && <HostsPage />}
            {active === "ssh-config" && <SshPage />}
            {active === "wsl" && <WslPage />}
            {active === "proxy-settings" && <ProxyPage />}
            {active === "code-signing" && <CodeSigningPage />}
            {active === "certificate-store" && <CertificatesPage />}
          </main>
        </SidebarInset>
        <CommandPalette
          open={paletteOpen}
          onOpenChange={setPaletteOpen}
          onNavigate={navigate}
        />
      </SidebarProvider>
    </div>
  );
}

export default App;
