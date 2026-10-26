import { useEffect, useState } from "react";
import { onProgress } from "@/lib/api";
import type { Progress } from "@/lib/types";

/** Latest progress event while `active` is true. */
export function useProgress(active: boolean): Progress | null {
  const [p, setP] = useState<Progress | null>(null);
  useEffect(() => {
    if (!active) {
      setP(null);
      return;
    }
    let off: (() => void) | undefined;
    let cancelled = false;
    onProgress(setP).then((u) => (cancelled ? u() : (off = u)));
    return () => {
      cancelled = true;
      off?.();
    };
  }, [active]);
  return p;
}
