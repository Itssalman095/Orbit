import { useState } from "react";
import { ipc } from "../lib/ipc";

export type PickKind = "executable" | "icon" | "directory";

export interface ApplicationPickerProps {
  kind: PickKind;
  label?: string;
  title?: string;
  onPicked: (path: string) => void;
  disabled?: boolean;
  className?: string;
}

/**
 * Opens the native Windows file/folder picker and returns the chosen path.
 * Used for adding executables, choosing custom icons and working directories.
 */
export function ApplicationPicker({
  kind,
  label,
  title,
  onPicked,
  disabled,
  className,
}: ApplicationPickerProps) {
  const [busy, setBusy] = useState(false);

  const pick = async () => {
    if (busy) return;
    setBusy(true);
    try {
      const path =
        kind === "executable"
          ? await ipc.pickExecutable()
          : kind === "icon"
            ? await ipc.pickIcon()
            : await ipc.pickDirectory();
      if (path && path.length > 0) onPicked(path);
    } catch (e) {
      console.error("file picker failed", e);
    } finally {
      setBusy(false);
    }
  };

  const defaultLabel =
    kind === "executable" ? "Add application…" : kind === "icon" ? "Choose icon…" : "Browse…";

  return (
    <button
      type="button"
      className={"btn" + (className ? ` ${className}` : "")}
      onClick={pick}
      disabled={disabled || busy}
      title={title ?? defaultLabel}
    >
      {label ?? defaultLabel}
    </button>
  );
}
