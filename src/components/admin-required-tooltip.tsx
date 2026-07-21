import { useTranslation } from "react-i18next";

import { Tooltip, TooltipContent, TooltipTrigger } from "@/components/ui/tooltip";

// Wraps an already-disabled admin-only control so hovering it still
// explains why — native `disabled` buttons don't fire the pointer/focus
// events tooltips rely on, so the trigger has to be a plain span around it.
export function AdminRequiredTooltip({
  locked,
  children,
}: {
  locked: boolean;
  children: React.ReactNode;
}) {
  const { t } = useTranslation();

  if (!locked) return <>{children}</>;

  return (
    <Tooltip>
      <TooltipTrigger render={<span className="inline-flex" />}>
        {children}
      </TooltipTrigger>
      <TooltipContent>{t("common.requiresAdministrator")}</TooltipContent>
    </Tooltip>
  );
}
