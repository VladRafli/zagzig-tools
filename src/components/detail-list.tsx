import { useTranslation } from "react-i18next";

import { cn } from "@/lib/utils";

export type DetailValue = string | string[] | boolean | number | null | undefined;

export function DetailRow({
  label,
  value,
}: {
  label: string;
  value: DetailValue;
}) {
  const { t } = useTranslation();

  let display: string;
  if (value === null || value === undefined || value === "") {
    display = t("common.none");
  } else if (Array.isArray(value)) {
    display = value.length === 0 ? t("common.none") : value.join(", ");
  } else if (typeof value === "boolean") {
    display = value ? t("common.yes") : t("common.no");
  } else {
    display = String(value);
  }

  return (
    <>
      <dt className="text-muted-foreground">{label}</dt>
      <dd className="break-words">{display}</dd>
    </>
  );
}

export function DetailList({
  className,
  ...props
}: React.ComponentProps<"dl">) {
  return (
    <dl
      className={cn(
        "grid grid-cols-[max-content_1fr] gap-x-4 gap-y-1.5 text-sm sm:grid-cols-[max-content_1fr_max-content_1fr]",
        className,
      )}
      {...props}
    />
  );
}
