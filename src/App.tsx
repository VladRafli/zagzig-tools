import { useState } from "react";
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
import { UpdateButton } from "@/components/update-button";
import { CertificatesPage } from "@/features/certificates/certificates-page";
import { CodeSigningPage } from "@/features/code-signing/code-signing-page";
import { ConnectionTestPage } from "@/features/connection-test/connection-test-page";
import { DashboardPage } from "@/features/dashboard/dashboard-page";
import { DnsPage } from "@/features/dns/dns-page";
import { DnsCachePage } from "@/features/dns-cache/dns-cache-page";
import { DnsMonitorPage } from "@/features/dns-monitor/dns-monitor-page";
import { HostsPage } from "@/features/hosts/hosts-page";
import { NrptRulesPage } from "@/features/nrpt/nrpt-rules-page";
import { ProxyPage } from "@/features/proxy/proxy-page";
import { RoutingPage } from "@/features/routing/routing-page";
import { navGroups, type NavId } from "@/lib/nav";

function App() {
  const { t } = useTranslation();
  const [active, setActive] = useState<NavId>("dashboard");

  return (
    <SidebarProvider>
      <Sidebar>
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
      <SidebarInset>
        <header className="flex h-12 items-center gap-2 border-b px-4">
          <SidebarTrigger />
          <div className="ml-auto">
            <AdminStatusBadge />
          </div>
        </header>
        <main className="p-6">
          {active === "dashboard" && <DashboardPage onNavigate={setActive} />}
          {active === "nrpt-rules" && <NrptRulesPage />}
          {active === "connection-test" && <ConnectionTestPage />}
          {active === "network-routes" && <RoutingPage />}
          {active === "dns-servers" && <DnsPage />}
          {active === "dns-cache" && <DnsCachePage />}
          {active === "dns-monitor" && <DnsMonitorPage />}
          {active === "hosts-file" && <HostsPage />}
          {active === "proxy-settings" && <ProxyPage />}
          {active === "code-signing" && <CodeSigningPage />}
          {active === "certificate-store" && <CertificatesPage />}
        </main>
      </SidebarInset>
    </SidebarProvider>
  );
}

export default App;
