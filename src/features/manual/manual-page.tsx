import { useMemo, useState, type ComponentProps } from "react";
import { openUrl } from "@tauri-apps/plugin-opener";
import { Search } from "lucide-react";
import Markdown, { type Components } from "react-markdown";
import rehypeSlug from "rehype-slug";
import remarkGfm from "remark-gfm";
import { useTranslation } from "react-i18next";

import { Input } from "@/components/ui/input";
import { parseSections } from "@/features/manual/sections";
import { cn } from "@/lib/utils";
// The same file the repository ships, so the in-app copy can't drift from it.
import manualSource from "../../../docs/user-manual.md?raw";

const REPO_BLOB_URL = "https://github.com/VladRafli/zagzig-tools/blob/main/";

function scrollToId(id: string) {
  document.getElementById(decodeURIComponent(id))?.scrollIntoView({
    behavior: "smooth",
    block: "start",
  });
}

function resolveHref(href: string): string {
  if (/^(https?:|mailto:)/.test(href)) return href;
  // Relative links point at files in the repository (../README.md, ...).
  return REPO_BLOB_URL + href.replace(/^(\.\.?\/)+/, "");
}

const heading = (Tag: "h1" | "h2" | "h3" | "h4", className: string) =>
  function Heading({ className: extra, ...props }: ComponentProps<typeof Tag>) {
    return <Tag {...props} className={cn("scroll-mt-4 font-semibold", className, extra)} />;
  };

const components: Components = {
  h1: heading("h1", "mb-3 text-2xl"),
  h2: heading("h2", "mt-10 mb-3 border-b pb-1 text-xl"),
  h3: heading("h3", "mt-7 mb-2 text-base"),
  h4: heading("h4", "mt-5 mb-2 text-sm"),
  p: ({ className, ...props }) => (
    <p {...props} className={cn("my-3 text-sm leading-relaxed", className)} />
  ),
  ul: ({ className, ...props }) => (
    <ul {...props} className={cn("my-3 list-disc space-y-1.5 pl-6 text-sm leading-relaxed", className)} />
  ),
  ol: ({ className, ...props }) => (
    <ol {...props} className={cn("my-3 list-decimal space-y-1.5 pl-6 text-sm leading-relaxed", className)} />
  ),
  li: ({ className, ...props }) => <li {...props} className={cn("pl-1", className)} />,
  strong: ({ className, ...props }) => (
    <strong {...props} className={cn("font-semibold", className)} />
  ),
  code: ({ className, ...props }) => (
    <code
      {...props}
      className={cn("rounded bg-muted px-1 py-0.5 font-mono text-[0.85em]", className)}
    />
  ),
  hr: () => <hr className="my-8" />,
  blockquote: ({ className, ...props }) => (
    <blockquote
      {...props}
      className={cn("my-3 border-l-2 pl-4 text-sm text-muted-foreground", className)}
    />
  ),
  table: ({ className, ...props }) => (
    <div className="my-4 overflow-x-auto rounded-lg border">
      <table {...props} className={cn("w-full text-sm", className)} />
    </div>
  ),
  th: ({ className, ...props }) => (
    <th
      {...props}
      className={cn("border-b bg-muted/50 px-3 py-2 text-left text-xs font-medium", className)}
    />
  ),
  td: ({ className, ...props }) => (
    <td {...props} className={cn("border-b px-3 py-2 align-top last:border-b-0", className)} />
  ),
  a: ({ href, children, className, ...props }) => {
    const target = href ?? "";
    return (
      <a
        {...props}
        href={target}
        className={cn("text-primary underline underline-offset-2", className)}
        onClick={(e) => {
          e.preventDefault();
          if (target.startsWith("#")) scrollToId(target.slice(1));
          else if (target) void openUrl(resolveHref(target));
        }}
      >
        {children}
      </a>
    );
  },
};

export function ManualPage() {
  const { t, i18n } = useTranslation();
  const [query, setQuery] = useState("");

  const sections = useMemo(() => parseSections(manualSource), []);

  const visible = useMemo(() => {
    const needle = query.trim().toLowerCase();
    if (!needle) return sections;
    return sections.filter(
      (s) => s.title.toLowerCase().includes(needle) || s.text.includes(needle),
    );
  }, [sections, query]);

  return (
    <div className="flex flex-col gap-6 lg:flex-row lg:items-start">
      <aside className="flex flex-col gap-3 lg:sticky lg:top-0 lg:max-h-[calc(100vh-6rem)] lg:w-64 lg:shrink-0">
        <div className="relative">
          <Search className="pointer-events-none absolute top-1/2 left-2.5 size-4 -translate-y-1/2 text-muted-foreground" />
          <Input
            className="pl-8"
            placeholder={t("manual.searchPlaceholder")}
            value={query}
            onChange={(e) => setQuery(e.currentTarget.value)}
          />
        </div>
        <nav
          aria-label={t("manual.contents")}
          className="flex max-h-72 flex-col gap-0.5 overflow-y-auto text-sm lg:max-h-none"
        >
          {visible.length === 0 && (
            <p className="px-2 py-1 text-muted-foreground">{t("manual.noResults")}</p>
          )}
          {visible.map((s) => (
            <button
              key={s.id}
              type="button"
              onClick={() => scrollToId(s.id)}
              className={cn(
                "rounded-md px-2 py-1 text-left hover:bg-muted",
                s.level === 3 && !query ? "pl-5 text-muted-foreground" : "font-medium",
              )}
            >
              {s.title}
            </button>
          ))}
        </nav>
      </aside>

      <article className="min-w-0 max-w-3xl flex-1">
        {i18n.language !== "en" && (
          <p className="mb-4 rounded-lg border bg-muted/30 px-3 py-2 text-sm text-muted-foreground">
            {t("manual.englishOnly")}
          </p>
        )}
        <Markdown remarkPlugins={[remarkGfm]} rehypePlugins={[rehypeSlug]} components={components}>
          {manualSource}
        </Markdown>
      </article>
    </div>
  );
}
