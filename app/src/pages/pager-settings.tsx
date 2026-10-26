import { useEffect, useState } from "react";
import { Undo2 } from "lucide-react";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { Input } from "@/components/ui/input";
import { cn } from "@/lib/utils";
import { SECTIONS, formatValue, valueProblem, type SectionDef, type SettingDef } from "@/lib/pager-settings";
import { useStore } from "@/lib/store";

const SELECT = "h-8 w-full rounded-md border border-input bg-transparent px-2 text-sm shadow-xs outline-none focus-visible:border-ring focus-visible:ring-3 focus-visible:ring-ring/50 disabled:opacity-50";

/** A whole-number field that only reports values PPS accepts. */
function NumberField({ def, value, disabled, onChange }: { def: SettingDef; value: number; disabled: boolean; onChange: (v: number) => void }) {
  const [text, setText] = useState(String(value));
  useEffect(() => setText(String(value)), [value]);
  const n = /^\d+$/.test(text.trim()) ? parseInt(text, 10) : NaN;
  const problem = Number.isNaN(n) ? "Enter a number" : valueProblem(def, n);
  const unit = def.control.kind === "number" || def.control.kind === "zeroOr" ? def.control.unit : "";
  return (
    <div className="space-y-1">
      <div className="flex items-center gap-2">
        <Input
          aria-label={def.label}
          inputMode="numeric"
          disabled={disabled}
          className={cn("w-24 font-mono", problem && "border-destructive")}
          value={text}
          onChange={(e) => {
            setText(e.target.value);
            const v = /^\d+$/.test(e.target.value.trim()) ? parseInt(e.target.value, 10) : NaN;
            if (!Number.isNaN(v) && !valueProblem(def, v)) onChange(v);
          }}
          onBlur={() => problem && setText(String(value))}
        />
        <span className="text-xs text-muted-foreground">{unit}</span>
      </div>
      {problem && <div className="text-xs text-destructive">{problem}</div>}
    </div>
  );
}

function SettingRow({ def, get, onChange, dbVersion }: { def: SettingDef; get: (k: string) => number | undefined; onChange: (v: number) => void; dbVersion: string }) {
  const original = useStore((s) => s.profile?.summary.settings?.[def.key]);
  const pending = useStore((s) => s.pagerSettings[def.key]);
  const value = pending ?? original;
  if (value === undefined) return null;
  const unsupported = !!def.minDb && dbVersion < def.minDb;
  const disabled = unsupported || (def.enabledIf ? !def.enabledIf(get) : false);
  const changed = pending !== undefined;
  const c = def.control;
  const id = `ps-${def.key}`;
  return (
    <div className="grid grid-cols-[minmax(0,1fr)_14rem] items-start gap-4 py-2.5">
      <div className="min-w-0">
        <label htmlFor={id} className={cn("flex items-center gap-2 text-sm font-medium", disabled && "text-muted-foreground")}>
          {def.label}
          {changed && <span className="size-2 shrink-0 rounded-full bg-amber-500" title="Unsaved change" />}
        </label>
        {def.hint && <p className="text-xs text-muted-foreground">{def.hint}</p>}
        {unsupported && <p className="text-xs text-muted-foreground">Needs a pager with radio DB {def.minDb} or newer; this one has {dbVersion}.</p>}
        {changed && <p className="text-xs text-amber-700 dark:text-amber-400">Was {formatValue(def, original)}</p>}
      </div>
      <div>
        {c.kind === "toggle" && (
          <label className="flex h-8 items-center gap-2 text-sm">
            <input id={id} type="checkbox" className="size-4 accent-primary" disabled={disabled} checked={value === 1} onChange={(e) => onChange(e.target.checked ? 1 : 0)} />
            {value === 1 ? "On" : "Off"}
          </label>
        )}
        {c.kind === "select" && (
          <select id={id} className={SELECT} disabled={disabled} value={value} onChange={(e) => onChange(Number(e.target.value))}>
            {(c.options.some(([x]) => x === value) ? c.options : [[value, String(value)] as [number, string], ...c.options]).map(([x, label]) => (
              <option key={x} value={x}>
                {label}
              </option>
            ))}
          </select>
        )}
        {c.kind === "number" && <NumberField def={def} value={value} disabled={disabled} onChange={onChange} />}
        {c.kind === "zeroOr" && (
          <div className="space-y-2">
            <select id={id} className={SELECT} disabled={disabled} value={value === 0 ? "zero" : "other"} onChange={(e) => onChange(e.target.value === "zero" ? 0 : c.start)}>
              <option value="zero">{c.zeroLabel}</option>
              <option value="other">{c.otherLabel}</option>
            </select>
            {value !== 0 && <NumberField def={def} value={value} disabled={disabled} onChange={onChange} />}
          </div>
        )}
      </div>
    </div>
  );
}

function Section({ section, dbVersion }: { section: SectionDef; dbVersion: string }) {
  const original = useStore((s) => s.profile?.summary.settings ?? {});
  const pending = useStore((s) => s.pagerSettings);
  const setSetting = useStore((s) => s.setSetting);
  const undoSettings = useStore((s) => s.undoSettings);
  const get = (k: string) => pending[k] ?? original[k];
  const defs = section.settings.filter((d) => original[d.key] !== undefined);
  if (defs.length === 0) return null;
  const changed = defs.filter((d) => pending[d.key] !== undefined).map((d) => d.key);
  return (
    <Card>
      <CardHeader>
        <div className="flex items-start gap-3">
          <div className="min-w-0 flex-1 space-y-1.5">
            <CardTitle className="text-base">{section.title}</CardTitle>
            <CardDescription>{section.note}</CardDescription>
          </div>
          {changed.length > 0 && (
            <Button variant="ghost" size="xs" onClick={() => undoSettings(changed)}>
              <Undo2 />
              Undo
            </Button>
          )}
        </div>
      </CardHeader>
      <CardContent className="divide-y">
        {defs.map((d, i) => (
          <div key={d.key}>
            {d.group && d.group !== defs[i - 1]?.group && <div className={cn("pb-1 text-xs font-medium text-muted-foreground", i > 0 && "pt-4")}>{d.group}</div>}
            <SettingRow def={d} get={get} dbVersion={dbVersion} onChange={(v) => setSetting(d.key, v)} />
          </div>
        ))}
      </CardContent>
    </Card>
  );
}

export default function PagerSettingsPage() {
  const profile = useStore((s) => s.profile);
  if (!profile) return null;
  return (
    <div className="mx-auto max-w-3xl space-y-6 p-8">
      <div>
        <h1 className="text-xl font-semibold tracking-tight">Pager settings</h1>
        <p className="text-sm text-muted-foreground">Device-wide settings from this backup's profile, the ones Unication's software sets on its general settings pages. Changes are written with the rest of your edits.</p>
      </div>
      {SECTIONS.map((s) => (
        <Section key={s.id} section={s} dbVersion={profile.backup.db_version} />
      ))}
    </div>
  );
}
