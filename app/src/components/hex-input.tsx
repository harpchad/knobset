import { useEffect, useState } from "react";
import { Input } from "@/components/ui/input";
import { cn } from "@/lib/utils";

/** Hex number field: keeps the typed text, reports a value only when it parses and fits `max`. */
export function HexInput({
  value,
  max,
  digits,
  onChange,
  className,
  showDecimal,
  ...rest
}: {
  value: number;
  max: number;
  digits: number;
  onChange: (v: number) => void;
  className?: string;
  /** Show the decimal value inside the field, e.g. "= 24" (RadioReference lists both). */
  showDecimal?: boolean;
  "aria-label"?: string;
}) {
  const fmt = (v: number) => v.toString(16).toUpperCase().padStart(digits, "0");
  const [text, setText] = useState(fmt(value));
  useEffect(() => setText((t) => (parseInt(t, 16) === value ? t : fmt(value))), [value]); // eslint-disable-line react-hooks/exhaustive-deps
  const n = /^[0-9a-fA-F]+$/.test(text) ? parseInt(text, 16) : NaN;
  const bad = Number.isNaN(n) || n > max;
  const input = (
    <Input
      {...rest}
      value={text}
      maxLength={digits}
      spellCheck={false}
      aria-invalid={bad}
      title={bad ? undefined : `Hex ${fmt(n)} = decimal ${n}`}
      className={cn("font-mono uppercase", showDecimal && "pr-12", className)}
      onChange={(e) => {
        const t = e.target.value.toUpperCase();
        setText(t);
        const v = /^[0-9A-F]+$/.test(t) ? parseInt(t, 16) : NaN;
        if (!Number.isNaN(v) && v <= max) onChange(v);
      }}
      onBlur={() => !bad && setText(fmt(n))}
    />
  );
  if (!showDecimal) return input;
  return (
    <div className="relative">
      {input}
      <span className="pointer-events-none absolute top-1/2 right-2 -translate-y-1/2 font-mono text-[11px] tabular-nums text-muted-foreground" aria-hidden>
        {bad ? "" : `= ${n}`}
      </span>
    </div>
  );
}

/** Frequency field in MHz (5 decimals); reports whole Hz. */
export function MhzInput({ hz, onChange, invalid, className, ...rest }: { hz: number; onChange: (hz: number) => void; invalid?: boolean; className?: string; id?: string; "aria-label"?: string }) {
  const [text, setText] = useState((hz / 1e6).toFixed(5));
  useEffect(() => setText((t) => (Math.round(parseFloat(t) * 1e6) === hz ? t : (hz / 1e6).toFixed(5))), [hz]);
  const v = /^\d{1,4}(\.\d{0,5})?$/.test(text) ? Math.round(parseFloat(text) * 1e6) : NaN;
  return (
    <Input
      {...rest}
      value={text}
      inputMode="decimal"
      spellCheck={false}
      aria-invalid={invalid || Number.isNaN(v)}
      className={cn("font-mono tabular-nums", className)}
      onChange={(e) => {
        setText(e.target.value);
        const n = /^\d{1,4}(\.\d{0,5})?$/.test(e.target.value) ? Math.round(parseFloat(e.target.value) * 1e6) : NaN;
        if (!Number.isNaN(n)) onChange(n);
      }}
      onBlur={() => !Number.isNaN(v) && setText((v / 1e6).toFixed(5))}
    />
  );
}
