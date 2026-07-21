import type { TFunction } from "i18next";

export function formatRelativeTime(t: TFunction, timestamp: number): string {
  const diffSec = Math.round((Date.now() - timestamp) / 1000);
  if (diffSec < 5) return t("common.justNow");
  if (diffSec < 60) return t("common.secondsAgo", { count: diffSec });

  const diffMin = Math.round(diffSec / 60);
  if (diffMin < 60) return t("common.minutesAgo", { count: diffMin });

  const diffHour = Math.round(diffMin / 60);
  if (diffHour < 24) return t("common.hoursAgo", { count: diffHour });

  const diffDay = Math.round(diffHour / 24);
  return t("common.daysAgo", { count: diffDay });
}
