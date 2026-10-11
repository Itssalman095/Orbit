/**
 * Loads icon data-URLs for apps through the Rust extractor, with a
 * module-level cache so switching tabs doesn't re-extract.
 */

import { useEffect, useState } from "react";
import { ipc } from "../lib/ipc";
import type { LauncherApp } from "../types/config";

const iconCache = new Map<string, Promise<string | null>>();

function fetchIcon(path: string): Promise<string | null> {
  let p = iconCache.get(path);
  if (!p) {
    p = ipc.getAppIcon(path).catch(() => null);
    iconCache.set(path, p);
  }
  return p;
}

export function useAppIcons(apps: LauncherApp[]): Record<string, string | null> {
  const [icons, setIcons] = useState<Record<string, string | null>>({});
  const key = apps.map((a) => `${a.id}:${a.iconPath ?? a.executablePath}`).join("|");

  useEffect(() => {
    let cancelled = false;
    const targets = apps.map((a) => ({ id: a.id, path: a.iconPath ?? a.executablePath }));
    void Promise.all(
      targets.map(async (t) => [t.id, await fetchIcon(t.path)] as const),
    ).then((entries) => {
      if (!cancelled) setIcons(Object.fromEntries(entries));
    });
    return () => {
      cancelled = true;
    };
  }, [key]);

  return icons;
}
