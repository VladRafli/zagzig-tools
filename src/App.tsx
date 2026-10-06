import { Suspense, lazy, useState } from "react";
import { useTranslation } from "react-i18next";

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
  SidebarMenuButton,
  SidebarMenuItem,
  SidebarProvider,
  SidebarTrigger,
} from "@/components/ui/sidebar";
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

// The manual pulls in a Markdown renderer, so it loads only when opened.
const ManualPage = lazy(() =>
  import("@/features/manual/manual-page").then((m) => ({ default: m.ManualPage })),
);
import { NrptRulesPage } from "@/features/nrpt/nrpt-rules-page";
import { ProxyPage } from "@/features/proxy/proxy-page";
import { RoutingPage } from "@/features/routing/routing-page";
import { navGroups, type NavId } from "@/lib/nav";

function App() {
  const { t } = useTranslation();
  const [active, setActive] = useState<NavId>("dashboard");

  return (
    <div className="flex h-screen flex-col overflow-hidden">
      <TitleBar />
      {/* [contain:layout] makes this the containing block for Sidebar's
          `fixed inset-y-0` panel, so it's positioned relative to the space
          below the title bar instead of the real viewport (which would put
          it behind the title bar). */}
      <SidebarProvider className="min-h-0 flex-1 [contain:layout]">
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
            {navGroups.map((group) => (
              <SidebarGroup key={group.labelKey}>
                <SidebarGroupLabel>{t(group.labelKey)}</SidebarGroupLabel>
                <SidebarGroupContent>
                  <SidebarMenu>
                    {group.items.map((item) => (
                      <SidebarMenuItem key={item.id}>
                        <SidebarMenuButton
                          isActive={active === item.id}
                          onClick={() => setActive(item.id)}
                        >
                          <item.icon />
                          <span>{t(item.labelKey)}</span>
                        </SidebarMenuButton>
                      </SidebarMenuItem>
                    ))}
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
            <div className="ml-auto">
              <AdminStatusBadge />
            </div>
          </header>
          <main className="min-h-0 flex-1 overflow-y-auto p-6">
            {active === "dashboard" && <DashboardPage onNavigate={setActive} />}
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
      </SidebarProvider>
    </div>
  );
}

export default App;
