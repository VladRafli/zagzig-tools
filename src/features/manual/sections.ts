import GithubSlugger from "github-slugger";

export interface Section {
  id: string;
  title: string;
  level: 2 | 3;
  // Lower-cased body text, used only for searching.
  text: string;
}

// Mirrors what the markdown renderer shows as heading text, so the slug
// matches the id `rehype-slug` gives the rendered heading.
function plainHeading(raw: string): string {
  return raw
    .replace(/\[([^\]]*)\]\([^)]*\)/g, "$1")
    .replace(/[`*_]/g, "")
    .trim();
}

// Splits the manual into its h2/h3 sections for the table of contents and
// search. Every heading level takes a slug (as in the renderer) so that
// duplicate titles get the same -1, -2 suffixes there; only h2/h3 become
// entries.
export function parseSections(markdown: string): Section[] {
  const slugger = new GithubSlugger();
  const sections: Section[] = [];
  let current: Section | null = null;
  let inFence = false;

  for (const line of markdown.split(/\r?\n/)) {
    if (/^\s*```/.test(line)) inFence = !inFence;
    const match = inFence ? null : /^(#{1,6})\s+(.*?)\s*#*\s*$/.exec(line);
    if (match) {
      const title = plainHeading(match[2]);
      const id = slugger.slug(title);
      const level = match[1].length;
      current = level === 2 || level === 3 ? { id, title, level, text: "" } : null;
      if (current) sections.push(current);
    } else if (current) {
      current.text += ` ${line.toLowerCase()}`;
    }
  }
  return sections;
}
