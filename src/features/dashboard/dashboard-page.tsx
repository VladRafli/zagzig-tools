import { ArrowRight, Network, Radar, Route, Server } from "lucide-react";
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
import { useSystemNrptRules } from "@/features/nrpt/use-system-nrpt-rules";
import { useNetworkRoutes } from "@/features/routing/use-network-routes";
import { useDnsSettings } from "@/features/dns/use-dns-settings";
import { CurrentUserCard } from "@/features/user/current-user-card";
import { AdminAlertCard } from "@/features/dashboard/admin-alert-card";
import type { NavId } from "@/lib/nav";

function nrptSummary(
  t: TFunction,
  status: "loading" | "ready" | "error",
  count: number,
) {
  if (status === "loading") return t("dashboard.nrpt.checking");
  if (status === "error") return t("dashboard.nrpt.couldntRead");
  if (count === 0) return t("dashboard.nrpt.noneConfigured");
  return t("dashboard.nrpt.configured", { count });
}

function routingSummary(
  t: TFunction,
  status: "loading" | "ready" | "error",
  count: number,
) {
  if (status === "loading") return t("dashboard.routing.checking");
  if (status === "error") return t("dashboard.routing.couldntRead");
  if (count === 0) return t("dashboard.routing.noneConfigured");
  return t("dashboard.routing.configured", { count });
}

function dnsSummary(
  t: TFunction,
  status: "loading" | "ready" | "error",
  count: number,
) {
  if (status === "loading") return t("dashboard.dns.checking");
  if (status === "error") return t("dashboard.dns.couldntRead");
  if (count === 0) return t("dashboard.dns.noneConfigured");
  return t("dashboard.dns.configured", { count });
}

export function DashboardPage({
  onNavigate,
}: {
  onNavigate: (id: NavId) => void;
}) {
  const { t } = useTranslation();
  const nrptRules = useSystemNrptRules();
  const routes = useNetworkRoutes();
  const dns = useDnsSettings();

  return (
    <div className="flex flex-col gap-6">
      <div>
        <h1 className="text-lg font-semibold">{t("dashboard.title")}</h1>
        <p className="text-sm text-muted-foreground">
          {t("dashboard.subtitle")}
        </p>
      </div>

      <AdminAlertCard />

      <CurrentUserCard />

      <div className="grid gap-4 sm:grid-cols-2 lg:grid-cols-3">
        <Card>
          <CardHeader>
            <div className="flex items-center gap-2">
              <Network className="size-4" />
              <CardTitle>{t("dashboard.nrpt.title")}</CardTitle>
            </div>
            <CardDescription>{t("dashboard.nrpt.description")}</CardDescription>
          </CardHeader>
          <CardContent className="flex items-center justify-between">
            <span className="text-sm text-muted-foreground">
              {nrptSummary(t, nrptRules.status, nrptRules.rules.length)}
            </span>
            <Button
              variant="outline"
              size="sm"
              onClick={() => onNavigate("nrpt-rules")}
            >
              {t("dashboard.open")}
              <ArrowRight />
            </Button>
          </CardContent>
        </Card>

        <Card>
          <CardHeader>
            <div className="flex items-center gap-2">
              <Radar className="size-4" />
              <CardTitle>{t("dashboard.connectionTest.title")}</CardTitle>
            </div>
            <CardDescription>
              {t("dashboard.connectionTest.description")}
            </CardDescription>
          </CardHeader>
          <CardContent className="flex items-center justify-end">
            <Button
              variant="outline"
              size="sm"
              onClick={() => onNavigate("connection-test")}
            >
              {t("dashboard.open")}
              <ArrowRight />
            </Button>
          </CardContent>
        </Card>

        <Card>
          <CardHeader>
            <div className="flex items-center gap-2">
              <Route className="size-4" />
              <CardTitle>{t("dashboard.routing.title")}</CardTitle>
            </div>
            <CardDescription>{t("dashboard.routing.description")}</CardDescription>
          </CardHeader>
          <CardContent className="flex items-center justify-between">
            <span className="text-sm text-muted-foreground">
              {routingSummary(t, routes.status, routes.routes.length)}
            </span>
            <Button
              variant="outline"
              size="sm"
              onClick={() => onNavigate("network-routes")}
            >
              {t("dashboard.open")}
              <ArrowRight />
            </Button>
          </CardContent>
        </Card>

        <Card>
          <CardHeader>
            <div className="flex items-center gap-2">
              <Server className="size-4" />
              <CardTitle>{t("dashboard.dns.title")}</CardTitle>
            </div>
            <CardDescription>{t("dashboard.dns.description")}</CardDescription>
          </CardHeader>
          <CardContent className="flex items-center justify-between">
            <span className="text-sm text-muted-foreground">
              {dnsSummary(t, dns.status, dns.interfaces.length)}
            </span>
            <Button
              variant="outline"
              size="sm"
              onClick={() => onNavigate("dns-servers")}
            >
              {t("dashboard.open")}
              <ArrowRight />
            </Button>
          </CardContent>
        </Card>
      </div>
    </div>
  );
}
