import { useEffect, useMemo, useRef, useState } from "react";
import { Search, Star } from "lucide-react";
import { useTranslation } from "react-i18next";

import { Dialog, DialogContent, DialogDescription, DialogTitle } from "@/components/ui/dialog";
import { cn } from "@/lib/utils";
import { navGroups, type NavId } from "@/lib/nav";
import { useStarred } from "@/lib/use-starred";

interface Entry {
  id: NavId;
  label: string;
  group: string;
  icon: (typeof navGroups)[number]["items"][number]["icon"];
}

// How well `query` matches `text`: 3 for a prefix of the text or of one of
// its words, 2 for a substring, 1 for letters in order (a fuzzy match like
// "dnsm" for "DNS Monitor"), 0 for no match.
export function matchScore(text: string, query: string): number {
  const t = text.toLowerCase();
  const q = query.toLowerCase();
  if (!q) return 1;
  if (t.startsWith(q) || t.split(/[\s()/-]+/).some((w) => w.startsWith(q))) return 3;
  if (t.includes(q)) return 2;
  let i = 0;
  for (const c of t) {
    if (c === q[i]) i++;
    if (i === q.length) return 1;
  }
  return 0;
}

export function CommandPalette({
  open,
  onOpenChange,
  onNavigate,
}: {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  onNavigate: (id: NavId) => void;
}) {
  const { t } = useTranslation();
  const starred = useStarred();
  const [query, setQuery] = useState("");
  const [active, setActive] = useState(0);
  const listRef = useRef<HTMLDivElement>(null);

  const entries = useMemo<Entry[]>(
    () =>
      navGroups.flatMap((g) =>
        g.items.map((i) => ({
          id: i.id,
          label: t(i.labelKey),
          group: t(g.labelKey),
          icon: i.icon,
        })),
      ),
    [t],
  );

  const results = useMemo(() => {
    const q = query.trim();
    if (!q) {
      // Starred features first, then everything in sidebar order.
      const first = starred
        .map((id) => entries.find((e) => e.id === id))
        .filter((e): e is Entry => !!e);
      return [...first, ...entries.filter((e) => !starred.includes(e.id))];
    }
    return entries
      .map((e) => ({
        e,
        score: Math.max(matchScore(e.label, q), matchScore(`${e.group} ${e.label}`, q) - 1, matchScore(e.id, q) - 1),
      }))
      .filter((r) => r.score > 0)
      .sort((a, b) => b.score - a.score)
      .map((r) => r.e);
  }, [entries, query, starred]);

  useEffect(() => {
    if (open) {
      setQuery("");
      setActive(0);
    }
  }, [open]);

  useEffect(() => {
    listRef.current
      ?.querySelector<HTMLElement>(`[data-index="${active}"]`)
      ?.scrollIntoView({ block: "nearest" });
  }, [active]);

  function choose(entry: Entry | undefined) {
    if (!entry) return;
    onOpenChange(false);
    onNavigate(entry.id);
  }

  function onKeyDown(event: React.KeyboardEvent) {
    if (event.key === "ArrowDown") {
      event.preventDefault();
      setActive((i) => Math.min(i + 1, results.length - 1));
    } else if (event.key === "ArrowUp") {
      event.preventDefault();
      setActive((i) => Math.max(i - 1, 0));
    } else if (event.key === "Enter") {
      event.preventDefault();
      choose(results[active]);
    }
  }

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent
        showCloseButton={false}
        className="top-[18%] translate-y-0 gap-0 p-0 sm:max-w-lg"
      >
        <DialogTitle className="sr-only">{t("palette.title")}</DialogTitle>
        <DialogDescription className="sr-only">{t("palette.description")}</DialogDescription>
        <div className="flex items-center gap-2 border-b px-3">
          <Search className="size-4 shrink-0 text-muted-foreground" />
          <input
            autoFocus
            value={query}
            onChange={(e) => {
              setQuery(e.currentTarget.value);
              setActive(0);
            }}
            onKeyDown={onKeyDown}
            placeholder={t("palette.placeholder")}
            aria-label={t("palette.placeholder")}
            role="combobox"
            aria-expanded="true"
            aria-controls="palette-list"
            className="h-11 w-full bg-transparent text-sm outline-none placeholder:text-muted-foreground"
          />
        </div>
        <div
          id="palette-list"
          role="listbox"
          ref={listRef}
          className="max-h-80 overflow-y-auto p-1"
        >
          {results.length === 0 && (
            <p className="px-3 py-6 text-center text-sm text-muted-foreground">
              {t("palette.empty")}
            </p>
          )}
          {results.map((entry, index) => (
            <div
              key={entry.id}
              role="option"
              aria-selected={index === active}
              data-index={index}
              onMouseMove={() => setActive(index)}
              onClick={() => choose(entry)}
              className={cn(
                "flex cursor-pointer items-center gap-2 rounded-md px-2 py-1.5 text-sm",
                index === active && "bg-accent text-accent-foreground",
              )}
            >
              <entry.icon className="size-4 shrink-0 text-muted-foreground" />
              <span className="flex-1 truncate">{entry.label}</span>
              {starred.includes(entry.id) && (
                <Star className="size-3.5 fill-current text-yellow-500" />
              )}
              <span className="text-xs text-muted-foreground">{entry.group}</span>
            </div>
          ))}
        </div>
        <p className="border-t px-3 py-1.5 text-xs text-muted-foreground">{t("palette.hint")}</p>
      </DialogContent>
    </Dialog>
  );
}
