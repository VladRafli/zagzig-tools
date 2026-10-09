// Snapshot shapes returned by the Rust `snapshots` module, and the diff
// between two of them. Kept free of React so it can be tested on its own.

export type Fields = Record<string, string>;
export type Items = Record<string, Fields>;

export interface Snapshot {
  version: number;
  id: string;
  label: string;
  takenAt: number;
  computer: string;
  sections: Record<string, Items>;
  errors: Record<string, string>;
}

export interface SnapshotInfo {
  id: string;
  label: string;
  takenAt: number;
  computer: string;
  itemCount: number;
  errorCount: number;
}

export const SECTION_ORDER = ["adapters", "routes", "nrpt", "proxy", "hosts"] as const;

export interface FieldChange {
  field: string;
  before: string | undefined;
  after: string | undefined;
}

export type ItemChange =
  | { kind: "added"; key: string; fields: Fields }
  | { kind: "removed"; key: string; fields: Fields }
  | { kind: "changed"; key: string; changes: FieldChange[] };

export interface SectionDiff {
  id: string;
  changes: ItemChange[];
  /** Set when the section couldn't be read in one of the two snapshots. */
  unreadable?: "before" | "after" | "both";
}

function diffFields(before: Fields, after: Fields): FieldChange[] {
  const names = [...new Set([...Object.keys(before), ...Object.keys(after)])].sort();
  return names
    .filter((field) => before[field] !== after[field])
    .map((field) => ({ field, before: before[field], after: after[field] }));
}

export function diffSnapshots(before: Snapshot, after: Snapshot): SectionDiff[] {
  const ids = [
    ...SECTION_ORDER,
    ...[...new Set([...Object.keys(before.sections), ...Object.keys(after.sections)])].filter(
      (id) => !(SECTION_ORDER as readonly string[]).includes(id),
    ),
  ];
  const out: SectionDiff[] = [];
  for (const id of ids) {
    const a = before.sections[id];
    const b = after.sections[id];
    if (!a || !b) {
      if (!a && !b && !before.errors?.[id] && !after.errors?.[id]) continue;
      out.push({ id, changes: [], unreadable: !a && !b ? "both" : !a ? "before" : "after" });
      continue;
    }
    const changes: ItemChange[] = [];
    const keys = [...new Set([...Object.keys(a), ...Object.keys(b)])].sort((x, y) =>
      x.localeCompare(y, undefined, { numeric: true }),
    );
    for (const key of keys) {
      if (!(key in a)) changes.push({ kind: "added", key, fields: b[key] });
      else if (!(key in b)) changes.push({ kind: "removed", key, fields: a[key] });
      else {
        const fieldChanges = diffFields(a[key], b[key]);
        if (fieldChanges.length) changes.push({ kind: "changed", key, changes: fieldChanges });
      }
    }
    out.push({ id, changes });
  }
  return out;
}

export function countChanges(diff: SectionDiff[]): number {
  return diff.reduce((n, s) => n + s.changes.length, 0);
}
