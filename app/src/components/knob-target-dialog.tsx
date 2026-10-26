import { useState } from "react";
import { ArrowLeftRight, Copy, MoveRight } from "lucide-react";
import { Button } from "@/components/ui/button";
import { Dialog, DialogContent, DialogDescription, DialogFooter, DialogHeader, DialogTitle } from "@/components/ui/dialog";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { MOVABLE_MODES, useStore } from "@/lib/store";
import { cn } from "@/lib/utils";
import type { Knob } from "@/lib/types";

const MAX_NAME = 14;
const SELECT = "h-8 w-full rounded-md border border-input bg-transparent px-2 text-sm shadow-xs outline-none focus-visible:border-ring focus-visible:ring-3 focus-visible:ring-ring/50";

function nameError(name: string): string | null {
  if (name.trim().length === 0) return "Name can't be empty";
  if (name.length > MAX_NAME) return `At most ${MAX_NAME} characters`;
  if (!/^[\x20-\x7E]+$/.test(name)) return "Letters, digits, spaces and basic punctuation only";
  return null;
}

type Slot = { pos: number; label: string; ok: boolean; why?: string; swap?: boolean };

/** Pick a zone and knob position to copy or move `knob` to. */
export function KnobTargetDialog({
  action,
  zone,
  knob,
  open,
  onOpenChange,
  onDone,
}: {
  action: "copy" | "move";
  zone: string;
  knob: Knob;
  open: boolean;
  onOpenChange: (o: boolean) => void;
  onDone: (zone: string, position: number) => void;
}) {
  const zonesView = useStore((s) => s.zonesView);
  const effectiveKnob = useStore((s) => s.effectiveKnob);
  const positionLock = useStore((s) => s.positionLock);
  const copyKnobTo = useStore((s) => s.copyKnobTo);
  const moveKnobTo = useStore((s) => s.moveKnobTo);
  // Re-render when anything pending changes what a position holds.
  useStore((s) => s.knobCopies);
  useStore((s) => s.knobMoves);
  useStore((s) => s.convKnobs);
  useStore((s) => s.knobSystems);
  useStore((s) => s.scanKnobs);
  useStore((s) => s.removedKnobs);
  const zones = zonesView();
  const [toZone, setToZone] = useState(zone);
  const [pos, setPos] = useState<number | null>(null);
  const [name, setName] = useState(knob.name);
  const target = zones.find((z) => z.name === toZone) ?? zones[0];

  const slots: Slot[] = Array.from({ length: 8 }, (_, i) => i + 1).map((p) => {
    const here = effectiveKnob(target, p);
    const lock = positionLock(target.name, p);
    if (target.name === zone && p === knob.index) return { pos: p, label: knob.name, ok: false, why: "This knob" };
    if (lock) return { pos: p, label: here?.name ?? "Empty", ok: false, why: `Position ${p} ${lock}` };
    if (!here) return target.empty_knobs.includes(p) ? { pos: p, label: "Empty", ok: true } : { pos: p, label: "Empty", ok: false, why: "Not an empty position Knobset can use" };
    if (action === "copy") return { pos: p, label: here.name, ok: false, why: "Copies go to empty positions; remove this knob first to replace it" };
    const stored = target.knobs.some((k) => k.index === p);
    return MOVABLE_MODES.includes(here.mode) && stored ? { pos: p, label: here.name, ok: true, swap: true } : { pos: p, label: here.name, ok: false, why: "Knobset can't move this kind of knob" };
  });
  const chosen = slots.find((s) => s.pos === pos && s.ok);
  const error = action === "copy" ? nameError(name) : null;

  function confirm() {
    if (!chosen || error) return;
    if (action === "copy") copyKnobTo(zone, knob.index, target.name, chosen.pos, name);
    else moveKnobTo(zone, knob.index, target.name, chosen.pos);
    onOpenChange(false);
    onDone(target.name, chosen.pos);
  }

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="sm:max-w-md">
        <DialogHeader>
          <DialogTitle>
            {action === "copy" ? "Copy" : "Move"} <span className="font-mono">{knob.name}</span>
          </DialogTitle>
          <DialogDescription>
            {action === "copy"
              ? "The copy gets the same channel, settings and voice prompt, as a new knob. A scan copy scans the same channels."
              : "Pick an empty position, or a knob to swap places with. The pager numbers channels by position, so the moved channels renumber; scans that list them follow."}
          </DialogDescription>
        </DialogHeader>
        <div className="space-y-4">
          <div className="space-y-1.5">
            <Label htmlFor="target-zone">Zone</Label>
            <select
              id="target-zone"
              className={SELECT}
              value={target.name}
              onChange={(e) => {
                setToZone(e.target.value);
                setPos(null);
              }}
            >
              {zones.map((z) => (
                <option key={z.id} value={z.name}>
                  {z.name}
                </option>
              ))}
            </select>
          </div>
          <div className="space-y-1.5">
            <Label id="target-pos-label">Position</Label>
            <div role="radiogroup" aria-labelledby="target-pos-label" className="grid grid-cols-4 gap-2">
              {slots.map((s) => (
                <button
                  key={s.pos}
                  type="button"
                  role="radio"
                  aria-checked={pos === s.pos}
                  disabled={!s.ok}
                  title={s.why ?? (s.swap ? `Swap with ${s.label}` : `Position ${s.pos}`)}
                  onClick={() => setPos(s.pos)}
                  className={cn(
                    "flex h-16 flex-col rounded-lg border p-2 text-left text-xs outline-none transition focus-visible:ring-3 focus-visible:ring-ring/50",
                    s.ok ? "hover:border-foreground/40" : "cursor-not-allowed opacity-45",
                    !s.swap && s.ok && "border-dashed",
                    pos === s.pos && "border-solid border-foreground bg-accent ring-2 ring-ring/50",
                  )}
                >
                  <span className="font-mono text-[11px] text-muted-foreground">{s.pos}</span>
                  <span className={cn("mt-auto truncate font-mono", s.label === "Empty" && "text-muted-foreground")}>{s.label}</span>
                </button>
              ))}
            </div>
            <p className="text-xs text-muted-foreground">{chosen ? (chosen.swap ? `Swaps with ${chosen.label}: it takes this knob's place.` : `Position ${chosen.pos} of ${target.name}.`) : "Choose a position."}</p>
          </div>
          {action === "copy" && (
            <div className="space-y-1.5">
              <Label htmlFor="copy-name">Name of the copy</Label>
              <Input id="copy-name" value={name} maxLength={MAX_NAME} className="font-mono" aria-invalid={!!error} onChange={(e) => setName(e.target.value)} />
              <div className="flex justify-between text-xs">
                <span className={error ? "text-destructive" : "text-muted-foreground"}>{error ?? "Unication's software keeps the original's name; change it here or later."}</span>
                <span className="tabular-nums text-muted-foreground">
                  {name.length}/{MAX_NAME}
                </span>
              </div>
            </div>
          )}
        </div>
        <DialogFooter>
          <Button variant="outline" onClick={() => onOpenChange(false)}>
            Cancel
          </Button>
          <Button disabled={!chosen || !!error} onClick={confirm}>
            {action === "copy" ? <Copy /> : chosen?.swap ? <ArrowLeftRight /> : <MoveRight />}
            {action === "copy" ? "Copy here" : chosen?.swap ? "Swap" : "Move here"}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
