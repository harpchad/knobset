import { Loader2 } from "lucide-react";
import { Progress as Bar } from "@/components/ui/progress";
import type { Progress } from "@/lib/types";

export function OpProgress({ progress, fallback }: { progress: Progress | null; fallback: string }) {
  const pct = progress && progress.total > 0 ? Math.round((progress.done / progress.total) * 100) : null;
  return (
    <div className="space-y-2">
      <div className="flex items-center gap-2 text-sm">
        <Loader2 className="size-4 animate-spin text-muted-foreground" />
        <span className="truncate">{progress?.message ?? fallback}</span>
        {pct !== null && <span className="ml-auto tabular-nums text-muted-foreground">{pct}%</span>}
      </div>
      <Bar value={pct ?? undefined} className={pct === null ? "animate-pulse" : undefined} />
    </div>
  );
}
