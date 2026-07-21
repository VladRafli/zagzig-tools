import { ShieldCheck, ShieldQuestion } from "lucide-react";
import { useTranslation } from "react-i18next";

import { Badge } from "@/components/ui/badge";
import { useIsAdministrator } from "@/lib/use-is-administrator";

export function AdminStatusBadge() {
  const { t } = useTranslation();
  const { isAdministrator, status } = useIsAdministrator();

  if (status === "loading") return null;

  return (
    <Badge
      variant={isAdministrator ? "default" : "secondary"}
      className="w-fit gap-1"
    >
      {isAdministrator ? (
        <ShieldCheck className="size-3" />
      ) : (
        <ShieldQuestion className="size-3" />
      )}
      {isAdministrator ? t("common.administrator") : t("common.standardUser")}
    </Badge>
  );
}
