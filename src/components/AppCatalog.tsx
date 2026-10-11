import { useEffect, useMemo, useRef, useState } from "react";
import { ipc } from "../lib/ipc";
import type { AvailableApp } from "../types/config";
import { AppIcon } from "./AppIcon";

export interface AppCatalogProps {
  onClose: () => void;
  /** Lowercased executable paths already configured; matching rows are dimmed. */
  addedPaths: Set<string>;
  onAdd: (path: string, name: string) => void;
}

// Module-level caches: reopening the catalog is instant and icons are
// never extracted twice per session.
let catalogCache: AvailableApp[] | null = null;
const iconPromiseCache = new Map<string, Promise<string | null>>();

const ICON_CONCURRENCY = 6;
const MAX_ICON_LOADS = 150;

async function pumpIcons(
  items: AvailableApp[],
  onIcon: (id: string, icon: string | null) => void,
) {
  let next = 0;
  const worker = async () => {
    while (next < items.length) {
      const item = items[next++];
      let p = iconPromiseCache.get(item.id);
      if (!p) {
        p = ipc.getAppIcon(item.id).catch(() => null);
        iconPromiseCache.set(item.id, p);
      }
      const icon = await p;
      onIcon(item.id, icon);
    }
  };
  await Promise.all(Array.from({ length: ICON_CONCURRENCY }, worker));
}

/**
 * "Add application" catalog: every installed app found in the Start Menu
 * and on the Desktop, searchable, with real icons. Clicking a row adds it
 * immediately — no file dialogs, no hunting for .exe paths.
 */
export function AppCatalog({ onClose, addedPaths, onAdd }: AppCatalogProps) {
  const [apps, setApps] = useState<AvailableApp[] | null>(catalogCache);
  const [icons, setIcons] = useState<Record<string, string | null>>({});
  const [query, setQuery] = useState("");
  const searchRef = useRef<HTMLInputElement>(null);

  // Load the catalog (cached after the first open).
  useEffect(() => {
    if (catalogCache) return;
    let cancelled = false;
    setApps(null);
    ipc
      .listAvailableApps()
      .then((list) => {
        catalogCache = list;
        if (!cancelled) setApps(list);
      })
      .catch(() => {
        if (!cancelled) setApps([]);
      });
    return () => {
      cancelled = true;
    };
  }, []);

  const filtered = useMemo(() => {
    if (!apps) return [];
    const q = query.trim().toLowerCase();
    if (q.length === 0) return apps;
    const starts: AvailableApp[] = [];
    const contains: AvailableApp[] = [];
    for (const app of apps) {
      const n = app.name.toLowerCase();
      if (n.startsWith(q)) starts.push(app);
      else if (n.includes(q)) contains.push(app);
    }
    return [...starts, ...contains];
  }, [apps, query]);

  // Progressive icon loading for the visible slice (promise-cached).
  useEffect(() => {
    if (filtered.length === 0) return;
    const targets = filtered.slice(0, MAX_ICON_LOADS);
    let cancelled = false;
    void pumpIcons(targets, (id, icon) => {
      if (!cancelled) setIcons((prev) => ({ ...prev, [id]: icon }));
    });
    return () => {
      cancelled = true;
    };
  }, [filtered]);

  // Keyboard: Escape closes, Enter adds the top match.
  useEffect(() => {
    searchRef.current?.focus();
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") {
        e.preventDefault();
        onClose();
      } else if (e.key === "Enter") {
        const top = filtered.find((a) => !addedPaths.has(a.path.toLowerCase()));
        if (top) {
          e.preventDefault();
          onAdd(top.path, top.name);
        }
      }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [filtered, addedPaths, onAdd, onClose]);

  const top = filtered.find((a) => !addedPaths.has(a.path.toLowerCase()));

  return (
    <div className="catalog-backdrop" onMouseDown={(e) => e.target === e.currentTarget && onClose()}>
      <div className="catalog-panel" role="dialog" aria-label="Add application">
        <div className="catalog-head">
          <div className="catalog-title">Add application</div>
          <input
            ref={searchRef}
            className="catalog-search"
            type="text"
            placeholder="Search installed apps…"
            value={query}
            onChange={(e) => setQuery(e.target.value)}
            spellCheck={false}
          />
        </div>

        <div className="catalog-list">
          {apps === null ? (
            <div className="catalog-empty">Scanning your installed apps…</div>
          ) : filtered.length === 0 ? (
            <div className="catalog-empty">
              {apps.length === 0
                ? "No installed apps were found. Use “Browse for a file…” below."
                : "No matches. Try another name or browse for a file."}
            </div>
          ) : (
            filtered.map((app) => {
              const added = addedPaths.has(app.path.toLowerCase());
              return (
                <button
                  type="button"
                  key={app.id}
                  className={"catalog-row" + (added ? " is-added" : "")}
                  onClick={() => onAdd(app.path, app.name)}
                  title={app.path}
                >
                  <span className="catalog-icon">
                    <AppIcon name={app.name} src={icons[app.id] ?? null} size={28} />
                  </span>
                  <span className="catalog-name">{app.name}</span>
                  <span className="catalog-check">{added ? "✓ added" : ""}</span>
                </button>
              );
            })
          )}
        </div>

        <div className="catalog-foot">
          <div className="catalog-hint">
            {top ? (
              <>
                Press <span className="kbd">Enter</span> to add <strong>{top.name}</strong>
              </>
            ) : (
              <>Pick apps from the list — they launch through their Start Menu shortcut.</>
            )}
          </div>
          <button type="button" className="btn" onClick={onClose}>
            Done
          </button>
        </div>
      </div>
    </div>
  );
}
