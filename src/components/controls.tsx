/**
 * Minimal, accessible form controls for the settings window.
 * Native elements wrapped in consistent styling; fully keyboard operable.
 */

import type { ReactNode } from "react";

export function Field({ label, hint, children }: { label: string; hint?: string; children: ReactNode }) {
  return (
    <label className="field">
      <div className="field-label">{label}</div>
      {children}
      {hint ? <div className="field-hint">{hint}</div> : null}
    </label>
  );
}

export function Toggle({
  checked,
  onChange,
  label,
  hint,
  disabled,
}: {
  checked: boolean;
  onChange: (value: boolean) => void;
  label: string;
  hint?: string;
  disabled?: boolean;
}) {
  return (
    <label className={"toggle" + (disabled ? " is-disabled" : "")}>
      <input
        type="checkbox"
        checked={checked}
        disabled={disabled}
        onChange={(e) => onChange(e.target.checked)}
      />
      <span className="toggle-track" aria-hidden="true">
        <span className="toggle-thumb" />
      </span>
      <span className="toggle-text">
        <span className="toggle-title">{label}</span>
        {hint ? <span className="toggle-hint">{hint}</span> : null}
      </span>
    </label>
  );
}

export function Slider({
  value,
  min,
  max,
  step = 1,
  onChange,
  label,
  format,
}: {
  value: number;
  min: number;
  max: number;
  step?: number;
  onChange: (value: number) => void;
  label: string;
  format?: (value: number) => string;
}) {
  return (
    <div className="field">
      <div className="field-label">
        {label}
        <span className="field-value">{format ? format(value) : value}</span>
      </div>
      <input
        type="range"
        min={min}
        max={max}
        step={step}
        value={value}
        onChange={(e) => onChange(Number(e.target.value))}
      />
    </div>
  );
}

export function Select<T extends string>({
  value,
  options,
  onChange,
  label,
  hint,
}: {
  value: T;
  options: { value: T; label: string }[];
  onChange: (value: T) => void;
  label: string;
  hint?: string;
}) {
  return (
    <label className="field">
      <div className="field-label">{label}</div>
      <select value={value} onChange={(e) => onChange(e.target.value as T)}>
        {options.map((o) => (
          <option key={o.value} value={o.value}>
            {o.label}
          </option>
        ))}
      </select>
      {hint ? <div className="field-hint">{hint}</div> : null}
    </label>
  );
}

export function TextField({
  value,
  onChange,
  label,
  placeholder,
  error,
  spellCheck = false,
  mono,
}: {
  value: string;
  onChange: (value: string) => void;
  label: string;
  placeholder?: string;
  error?: string | null;
  spellCheck?: boolean;
  mono?: boolean;
}) {
  return (
    <label className="field">
      <div className="field-label">{label}</div>
      <input
        type="text"
        className={mono ? "mono" : undefined}
        value={value}
        placeholder={placeholder}
        spellCheck={spellCheck}
        onChange={(e) => onChange(e.target.value)}
      />
      {error ? <div className="field-error">{error}</div> : null}
    </label>
  );
}
