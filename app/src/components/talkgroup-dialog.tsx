import { useState } from "react";
import { Trash2 } from "lucide-react";
import { Button } from "@/components/ui/button";
import { Dialog, DialogContent, DialogDescription, DialogFooter, DialogHeader, DialogTitle } from "@/components/ui/dialog";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import type { TalkgroupEdit } from "@/lib/types";
import { ALERT_MODES, ALERT_TONES, BACKGROUNDS, isBuiltinTone, settingsOf, toneAudible } from "@/lib/tg-settings";

const MAX_NAME = 14;
export const SELECT =
  "h-8 w-full rounded-md border border-input bg-transparent px-2 text-sm shadow-xs outline-none focus-visible:border-ring focus-visible:ring-3 focus-visible:ring-ring/50 disabled:opacity-50";

export function validTalkgroup(t: TalkgroupEdit, others: TalkgroupEdit[]): string | null {
  if (!t.name.trim()) return "Alias can't be blank";
  if (t.name.length > MAX_NAME) return `Alias is at most ${MAX_NAME} characters`;
  if (!/^[\x20-\x7E]+$/.test(t.name)) return "Alias: letters, digits, spaces and basic punctuation only";
  if (!Number.isInteger(t.tgid) || t.tgid < 1 || t.tgid > 65535) return "TGID must be 1 to 65535";
  const dup = others.find((o) => o.group_id !== t.group_id && o.tgid === t.tgid);
  if (dup) return `TGID ${t.tgid} is already used by ${dup.name}`;
  return null;
}

interface Props {
  open: boolean;
  onOpenChange: (o: boolean) => void;
  /** Existing talkgroup to edit, or a fresh one to add. */
  initial: TalkgroupEdit;
  isNew: boolean;
  all: TalkgroupEdit[];
  /** Knobs that list this talkgroup (shown before deleting). */
  usedOn: string[];
  onSave: (t: TalkgroupEdit) => void;
  onDelete?: () => void;
}

export function TalkgroupDialog({ open, onOpenChange, initial, isNew, all, usedOn, onSave, onDelete }: Props) {
  const [t, setT] = useState(initial);
  const [tgidText, setTgidText] = useState(isNew ? "" : String(initial.tgid));
  const [confirmDelete, setConfirmDelete] = useState(false);
  const error = validTalkgroup(t, all);
  const st = settingsOf(t);
  const customTone = !isBuiltinTone(st.alert_tone);

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="sm:max-w-md">
        <DialogHeader>
          <DialogTitle>{isNew ? "Add talkgroup" : `Edit ${initial.name}`}</DialogTitle>
          <DialogDescription>{isNew ? "Adds it to this system. Put it on a knob from Zones & knobs." : "Changes apply everywhere this talkgroup is used."}</DialogDescription>
        </DialogHeader>
        <div className="grid grid-cols-[6rem_1fr] items-center gap-x-3 gap-y-3">
          <Label htmlFor="tg-id">TGID</Label>
          <div className="flex items-center gap-2">
            <Input
              id="tg-id"
              autoFocus
              value={tgidText}
              inputMode="numeric"
              className="w-28 font-mono tabular-nums"
              onChange={(e) => {
                setTgidText(e.target.value);
                setT({ ...t, tgid: /^\d+$/.test(e.target.value) ? parseInt(e.target.value, 10) : NaN });
              }}
            />
            <span className="font-mono text-xs text-muted-foreground">{Number.isInteger(t.tgid) && t.tgid > 0 ? `hex ${t.tgid.toString(16).toUpperCase()}` : ""}</span>
          </div>
          <Label htmlFor="tg-name">Alias</Label>
          <div className="flex items-center gap-2">
            <Input id="tg-name" value={t.name} maxLength={MAX_NAME} className="font-mono" onChange={(e) => setT({ ...t, name: e.target.value })} />
            <span className="w-10 text-right text-xs tabular-nums text-muted-foreground">
              {t.name.length}/{MAX_NAME}
            </span>
          </div>
          <Label htmlFor="tg-note">Description</Label>
          <Input id="tg-note" value={t.note} onChange={(e) => setT({ ...t, note: e.target.value })} />
          <Label htmlFor="tg-alert">Alert</Label>
          <select id="tg-alert" className={SELECT} value={st.alert_mode} onChange={(e) => setT({ ...t, alert_mode: Number(e.target.value) })}>
            {!ALERT_MODES.some((m) => m.value === st.alert_mode) && <option value={st.alert_mode}>Mode {st.alert_mode} (as stored)</option>}
            {ALERT_MODES.map((m) => (
              <option key={m.value} value={m.value}>
                {m.label}
              </option>
            ))}
          </select>
          <Label htmlFor="tg-tone">Alert tone</Label>
          {customTone ? (
            <span id="tg-tone" className="text-sm text-muted-foreground" title={`AlertToneID ${st.alert_tone}`}>
              Custom tone from PPS (kept as is)
            </span>
          ) : (
            <select
              id="tg-tone"
              className={SELECT}
              value={st.alert_tone}
              disabled={!toneAudible(st.alert_mode)}
              title={toneAudible(st.alert_mode) ? undefined : "Plays only with Tone or Tone and vibrate"}
              onChange={(e) => setT({ ...t, alert_tone: e.target.value })}
            >
              {ALERT_TONES.map((x) => (
                <option key={x.value} value={x.value}>
                  {x.label}
                </option>
              ))}
            </select>
          )}
          <Label htmlFor="tg-storage">Voice storage</Label>
          <label className="flex items-center gap-2 text-sm">
            <input id="tg-storage" type="checkbox" className="accent-primary" checked={st.voice_storage} onChange={(e) => setT({ ...t, voice_storage: e.target.checked })} />
            Save calls for playback
          </label>
          <Label htmlFor="tg-bg">Background</Label>
          <div className="flex items-center gap-2">
            <select id="tg-bg" className={SELECT} value={st.background} onChange={(e) => setT({ ...t, background: Number(e.target.value) })}>
              {!BACKGROUNDS.some((b) => b.value === st.background) && <option value={st.background}>Colour {st.background} (as stored)</option>}
              {BACKGROUNDS.map((b) => (
                <option key={b.value} value={b.value}>
                  {b.label}
                </option>
              ))}
            </select>
            <Swatch value={st.background} />
          </div>
        </div>
        <p className={error ? "text-xs text-destructive" : "text-xs text-transparent"}>{error ?? "ok"}</p>
        {confirmDelete && (
          <div className="rounded-md border border-destructive/30 bg-destructive/5 p-3 text-sm">
            {usedOn.length > 0 ? `It will also be removed from ${usedOn.join(", ")}.` : "No knob uses it."} Delete {initial.name}?
          </div>
        )}
        <DialogFooter className="sm:justify-between">
          {!isNew && onDelete ? (
            <Button
              variant={confirmDelete ? "destructive" : "ghost"}
              onClick={() => {
                if (!confirmDelete) return setConfirmDelete(true);
                onDelete();
                onOpenChange(false);
              }}
            >
              <Trash2 />
              {confirmDelete ? "Delete" : "Delete…"}
            </Button>
          ) : (
            <span />
          )}
          <div className="flex gap-2">
            <Button variant="outline" onClick={() => onOpenChange(false)}>
              Cancel
            </Button>
            <Button
              disabled={!!error}
              onClick={() => {
                onSave(t);
                onOpenChange(false);
              }}
            >
              {isNew ? "Add" : "Apply"}
            </Button>
          </div>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}

export function Swatch({ value, className }: { value: number; className?: string }) {
  const b = BACKGROUNDS.find((x) => x.value === value);
  if (!b?.swatch) return <span className={className ?? "size-4 shrink-0 rounded-sm border border-dashed"} title={b?.label ?? `Colour ${value}`} />;
  return <span className={className ?? "size-4 shrink-0 rounded-sm border"} style={{ background: b.swatch }} title={b.label} />;
}
