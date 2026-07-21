import { Loader2, RefreshCw, User as UserIcon } from "lucide-react";
import { useTranslation } from "react-i18next";

import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Card, CardContent } from "@/components/ui/card";
import { DetailList, DetailRow } from "@/components/detail-list";
import { useCurrentUser } from "@/features/user/use-current-user";
import { formatRelativeTime } from "@/lib/relative-time";

function extractCn(dn: string | null): string | null {
  if (!dn) return null;
  const match = /^CN=([^,]+)/i.exec(dn);
  return match ? match[1] : dn;
}

function initialsOf(name: string): string {
  return name
    .split(/[\s.]+/)
    .filter(Boolean)
    .slice(0, 2)
    .map((part) => part[0]?.toUpperCase())
    .join("");
}

export function CurrentUserCard() {
  const { t } = useTranslation();
  const { user, status, error, updatedAt, refresh } = useCurrentUser();

  if (status === "loading" && !user) {
    return (
      <Card>
        <CardContent className="flex items-center gap-2 text-sm text-muted-foreground">
          <Loader2 className="size-4 animate-spin" />
          {t("user.reading")}
        </CardContent>
      </Card>
    );
  }

  if (!user) {
    return (
      <Card>
        <CardContent className="flex items-center justify-between gap-4">
          <span className="text-sm text-destructive">
            {t("user.couldntRead", { error })}
          </span>
          <Button variant="outline" size="sm" onClick={refresh}>
            <RefreshCw />
            {t("common.retry")}
          </Button>
        </CardContent>
      </Card>
    );
  }

  const ldap = user.ldap;
  const name = ldap?.displayName || user.userName;
  const initials = initialsOf(name);

  return (
    <Card>
      <CardContent className="flex flex-col gap-4">
        <div className="flex items-center gap-3">
          <div className="flex size-10 shrink-0 items-center justify-center rounded-full bg-primary text-sm font-medium text-primary-foreground">
            {initials || <UserIcon className="size-4" />}
          </div>
          <div className="flex flex-col">
            <span className="font-medium">{name}</span>
            <span className="text-sm text-muted-foreground">
              {user.domain}\{user.userName} · {user.computerName}
            </span>
          </div>
          <div className="ml-auto flex items-center gap-2">
            {updatedAt && (
              <span className="text-xs text-muted-foreground">
                {t("common.updatedAgo", {
                  time: formatRelativeTime(t, updatedAt),
                })}
              </span>
            )}
            <Button
              variant="ghost"
              size="icon-sm"
              onClick={refresh}
              disabled={status === "loading"}
            >
              <RefreshCw className={status === "loading" ? "animate-spin" : ""} />
            </Button>
            <Badge variant={user.isDomainJoined ? "default" : "secondary"}>
              {user.isDomainJoined ? t("user.domain") : t("user.local")}
            </Badge>
          </div>
        </div>

        {ldap ? (
          <DetailList>
            <DetailRow
              label={t("user.fields.email")}
              value={ldap.emailAddress}
            />
            <DetailRow label={t("user.fields.title")} value={ldap.title} />
            <DetailRow
              label={t("user.fields.department")}
              value={ldap.department}
            />
            <DetailRow
              label={t("user.fields.company")}
              value={ldap.company}
            />
            <DetailRow label={t("user.fields.office")} value={ldap.office} />
            <DetailRow
              label={t("user.fields.manager")}
              value={extractCn(ldap.manager)}
            />
            <DetailRow
              label={t("user.fields.phone")}
              value={ldap.voiceTelephoneNumber}
            />
            <DetailRow
              label={t("user.fields.accountEnabled")}
              value={ldap.enabled}
            />
            <DetailRow
              label={t("user.fields.upn")}
              value={ldap.userPrincipalName}
            />
            <DetailRow
              label={t("user.fields.distinguishedName")}
              value={ldap.distinguishedName}
            />
          </DetailList>
        ) : (
          <p className="text-sm text-muted-foreground">
            {user.ldapError
              ? t("user.couldntReadLdap", { error: user.ldapError })
              : t("user.notDomainJoined")}
          </p>
        )}
      </CardContent>
    </Card>
  );
}
