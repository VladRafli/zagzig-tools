import { Star } from "lucide-react";
import { useTranslation } from "react-i18next";

import { CurrentUserCard } from "@/features/user/current-user-card";
import { AdminAlertCard } from "@/features/dashboard/admin-alert-card";
import { cn } from "@/lib/utils";
import { navGroups, type NavItem, type NavId } from "@/lib/nav";
import { toggleStarred, useStarred } from "@/lib/use-starred";

function FeatureTile({
  item,
  starred,
  onOpen,
}: {
  item: NavItem;
  starred: boolean;
  onOpen: (id: NavId) => void;
}) {
  const { t } = useTranslation();
  const label = t(item.labelKey);
  const starLabel = t(starred ? "dashboard.unstar" : "dashboard.star", { name: label });
  return (
    <div className="group relative">
      <button
        type="button"
        onClick={() => onOpen(item.id)}
        className="flex h-24 w-full flex-col items-center gap-1.5 rounded-lg px-1 pt-3 text-center text-xs transition-colors hover:bg-accent focus-visible:outline-2 focus-visible:outline-ring"
      >
        <item.icon className="size-8 shrink-0 text-muted-foreground group-hover:text-foreground" />
        <span className="line-clamp-2 leading-tight">{label}</span>
      </button>
      <button
        type="button"
        onClick={() => toggleStarred(item.id)}
        aria-pressed={starred}
        aria-label={starLabel}
        title={starLabel}
        className={cn(
          "absolute top-1 right-1 rounded p-0.5 text-muted-foreground hover:text-foreground focus-visible:opacity-100",
          starred ? "opacity-100" : "opacity-0 group-hover:opacity-100",
        )}
      >
        <Star className={cn("size-3.5", starred && "fill-current text-yellow-500")} />
      </button>
    </div>
  );
}

function TileGrid({ children }: { children: React.ReactNode }) {
  return (
    <div className="grid grid-cols-[repeat(auto-fill,minmax(5.5rem,1fr))] gap-1">
      {children}
    </div>
  );
}

export function DashboardPage({
  onNavigate,
}: {
  onNavigate: (id: NavId) => void;
}) {
  const { t } = useTranslation();
  const starred = useStarred();

  const all = navGroups.flatMap((g) => g.items).filter((i) => i.id !== "dashboard");
  const starredItems = starred
    .map((id) => all.find((i) => i.id === id))
    .filter((i): i is NavItem => !!i);

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

      <section className="flex flex-col gap-2">
        <h2 className="text-sm font-medium text-muted-foreground">
          {t("dashboard.starred")}
        </h2>
        {starredItems.length === 0 ? (
          <p className="text-sm text-muted-foreground">{t("dashboard.starredEmpty")}</p>
        ) : (
          <TileGrid>
            {starredItems.map((item) => (
              <FeatureTile key={item.id} item={item} starred onOpen={onNavigate} />
            ))}
          </TileGrid>
        )}
      </section>

      {navGroups.map((group) => {
        const items = group.items.filter((item) => item.id !== "dashboard");
        return (
          <section key={group.labelKey} className="flex flex-col gap-2">
            <h2 className="text-sm font-medium text-muted-foreground">
              {t(group.labelKey)}
            </h2>
            <TileGrid>
              {items.map((item) => (
                <FeatureTile
                  key={item.id}
                  item={item}
                  starred={starred.includes(item.id)}
                  onOpen={onNavigate}
                />
              ))}
            </TileGrid>
          </section>
        );
      })}
    </div>
  );
}
