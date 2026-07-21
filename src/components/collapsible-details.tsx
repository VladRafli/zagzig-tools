import { useState } from "react";
import { ChevronDown } from "lucide-react";
import { useTranslation } from "react-i18next";

import { Button } from "@/components/ui/button";
import {
  Collapsible,
  CollapsibleContent,
  CollapsibleTrigger,
} from "@/components/ui/collapsible";

export function CollapsibleDetails({
  label,
  children,
}: {
  label?: string;
  children: React.ReactNode;
}) {
  const { t } = useTranslation();
  const [open, setOpen] = useState(false);
  const resolvedLabel = label ?? t("common.details");

  return (
    <Collapsible open={open} onOpenChange={setOpen}>
      <CollapsibleTrigger
        render={
          <Button variant="ghost" size="sm" className="-mx-2.5 w-fit">
            <ChevronDown
              className={
                open ? "rotate-180 transition-transform" : "transition-transform"
              }
            />
            {open
              ? t("common.hide", { label: resolvedLabel })
              : t("common.show", { label: resolvedLabel })}
          </Button>
        }
      />
      <CollapsibleContent>{children}</CollapsibleContent>
    </Collapsible>
  );
}
