import { cn } from "@/lib/utils";

export function Console({
  lines,
  className,
}: {
  lines: string[];
  className?: string;
}) {
  return (
    <pre
      className={cn(
        "overflow-x-auto rounded-lg bg-zinc-950 p-3 font-mono text-xs whitespace-pre text-zinc-100",
        className,
      )}
    >
      {lines.length === 0 ? (
        <span className="text-zinc-500">No output.</span>
      ) : (
        lines.map((line, i) => <div key={i}>{line || " "}</div>)
      )}
    </pre>
  );
}
