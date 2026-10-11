/**
 * Settings tab content sections. Each receives typed props from the
 * SettingsWindow shell, which owns the config state.
 */

import { useMemo, useState } from "react";
import { Field, Select, Slider, TextField, Toggle } from "../components/controls";
import { ApplicationPicker } from "../components/ApplicationPicker";
import { RadialMenu } from "../components/RadialMenu";
import { useAppIcons } from "../hooks/useAppIcons";
import { ipc } from "../lib/ipc";
import { nameFromPath, splitCommandLine, validateLauncherApp } from "../lib/configValidation";
import type { LauncherApp, MenuApp, Settings, TriggerButton } from "../types/config";

export interface SectionProps {
  settings: Settings;
  onSettingsChange: (patch: Partial<Settings>) => void;
}

export function GeneralSection({
  settings,
  onSettingsChange,
  onOpenAbout,
}: SectionProps & { onOpenAbout: () => void }) {
  return (
    <div className="section">
      <h2>General</h2>
      <p className="section-desc">Core behavior of the launcher.</p>

      <Toggle
        label="Enable launcher"
        hint="When enabled, holding the trigger button opens the radial menu anywhere in Windows."
        checked={settings.enabled}
        onChange={(v) => onSettingsChange({ enabled: v })}
      />

      <Select<TriggerButton>
        label="Activation button"
        value={settings.trigger}
        hint="While the launcher is enabled, this button is captured globally. Middle-click in other apps is disabled until you turn the launcher off."
        onChange={(v) => onSettingsChange({ trigger: v })}
        options={[
          { value: "middle", label: "Middle mouse button (wheel click)" },
          { value: "x1", label: "Mouse button 4 (X1)" },
          { value: "x2", label: "Mouse button 5 (X2)" },
          { value: "right", label: "Right mouse button (not recommended)" },
        ]}
      />

      <Toggle
        label="Start with Windows"
        hint="Registers a per-user Run entry (HKCU). No administrator rights required."
        checked={settings.startWithWindows}
        onChange={(v) => onSettingsChange({ startWithWindows: v })}
      />

      <Toggle
        label="Start minimized"
        hint="When RRRex starts automatically, stay in the tray without opening this window."
        checked={settings.startMinimized}
        disabled={!settings.startWithWindows}
        onChange={(v) => onSettingsChange({ startMinimized: v })}
      />

      <div className="callout">
        Closing this window keeps RRRex running in the system tray. Use{" "}
        <span className="kbd">Quit</span> in the tray menu to exit.
      </div>

      <button type="button" className="btn" onClick={onOpenAbout}>
        About RRRex
      </button>
    </div>
  );
}

export function AppearanceSection({
  settings,
  applications,
  onSettingsChange,
  onToast,
}: SectionProps & {
  applications: LauncherApp[];
  onToast: (msg: string) => void;
}) {
  const icons = useAppIcons(applications);
  const menuApps: MenuApp[] = useMemo(
    () =>
      applications
        .filter((a) => a.enabled)
        .map((a) => ({ id: a.id, name: a.name, icon: icons[a.id] ?? null })),
    [applications, icons],
  );

  return (
    <div className="section">
      <h2>Appearance</h2>
      <p className="section-desc">Menu geometry and visual style. The preview mirrors the real menu.</p>

      <div className="appearance-grid">
        <div className="appearance-controls">
          <Slider
            label="Menu radius"
            value={settings.radius}
            min={60}
            max={400}
            onChange={(v) => onSettingsChange({ radius: v })}
            format={(v) => `${v} px`}
          />
          <Slider
            label="Icon size"
            value={settings.iconSize}
            min={28}
            max={96}
            onChange={(v) => onSettingsChange({ iconSize: v })}
            format={(v) => `${v} px`}
          />
          <Slider
            label="Dead-zone radius"
            value={settings.deadZoneRadius}
            min={0}
            max={120}
            onChange={(v) => onSettingsChange({ deadZoneRadius: v })}
            format={(v) => `${v} px`}
          />
          <Slider
            label="Start angle"
            value={settings.startAngleDeg}
            min={-180}
            max={180}
            onChange={(v) => onSettingsChange({ startAngleDeg: v })}
            format={(v) => `${v}°`}
          />
          <Slider
            label="Animation speed"
            value={settings.animationSpeed}
            min={0}
            max={400}
            onChange={(v) => onSettingsChange({ animationSpeed: v })}
            format={(v) => (v === 0 ? "instant" : `${v} ms`)}
          />
          <Slider
            label="Menu opacity"
            value={Math.round(settings.menuOpacity * 100)}
            min={20}
            max={100}
            onChange={(v) => onSettingsChange({ menuOpacity: v / 100 })}
            format={(v) => `${v}%`}
          />
          <Field label="Theme">
            <div className="theme-choice">
              {(["dark", "light"] as const).map((t) => (
                <button
                  key={t}
                  type="button"
                  className={"btn theme-btn" + (settings.theme === t ? " is-active" : "")}
                  onClick={() => onSettingsChange({ theme: t })}
                >
                  {t === "dark" ? "Dark" : "Light"}
                </button>
              ))}
            </div>
          </Field>
          <Toggle
            label="Selection glow"
            checked={settings.selectionGlow}
            onChange={(v) => onSettingsChange({ selectionGlow: v })}
          />
        </div>

        <div className="preview-box" data-theme={settings.theme}>
          <RadialMenu
            interactive
            apps={menuApps}
            radius={Math.min(settings.radius, 160)}
            iconSize={Math.min(settings.iconSize, 44)}
            startAngleDeg={settings.startAngleDeg}
            animationSpeed={settings.animationSpeed}
            theme={settings.theme}
            menuOpacity={settings.menuOpacity}
            selectionGlow={settings.selectionGlow}
            selectedIndex={null}
            showDeadZone
            deadZoneRadius={Math.min(settings.deadZoneRadius, 60)}
            emptyHint="Add apps in the Applications tab"
            onPickIndex={(i) => onToast(`Would launch: ${menuApps[i]?.name ?? "?"}`)}
          />
          <div className="preview-note">Live preview — move the mouse inside</div>
        </div>
      </div>
    </div>
  );
}

export interface ApplicationsSectionProps {
  applications: LauncherApp[];
  missing: Record<string, boolean>;
  onApplicationsChange: (apps: LauncherApp[]) => void;
  onToast: (msg: string) => void;
}

export function ApplicationsSection({
  applications,
  missing,
  onApplicationsChange,
  onToast,
}: ApplicationsSectionProps) {
  const icons = useAppIcons(applications);

  const addApp = (path: string) => {
    if (applications.some((a) => a.executablePath.toLowerCase() === path.toLowerCase())) {
      onToast("That application is already in the list.");
      return;
    }
    const app: LauncherApp = {
      id: crypto.randomUUID(),
      name: nameFromPath(path),
      executablePath: path,
      enabled: true,
    };
    onApplicationsChange([...applications, app]);
  };

  const update = (id: string, patch: Partial<LauncherApp>) => {
    onApplicationsChange(applications.map((a) => (a.id === id ? { ...a, ...patch } : a)));
  };

  const remove = (id: string) => {
    onApplicationsChange(applications.filter((a) => a.id !== id));
  };

  const move = (index: number, delta: number) => {
    const next = [...applications];
    const j = index + delta;
    if (j < 0 || j >= next.length) return;
    [next[index], next[j]] = [next[j], next[index]];
    onApplicationsChange(next);
  };

  return (
    <div className="section">
      <div className="section-head">
        <div>
          <h2>Applications</h2>
          <p className="section-desc">
            Items shown in the radial menu, in order. Disabled apps are skipped.
          </p>
        </div>
        <ApplicationPicker kind="executable" label="Add application…" onPicked={addApp} />
      </div>

      {applications.length === 0 ? (
        <div className="empty-state">
          <div className="empty-title">No applications yet</div>
          <div>Add your favorite programs with the button above.</div>
        </div>
      ) : (
        <ul className="app-list">
          {applications.map((app, i) => (
            <AppRow
              key={app.id}
              app={app}
              index={i}
              total={applications.length}
              icon={icons[app.id] ?? null}
              isMissing={Boolean(missing[app.id])}
              onUpdate={update}
              onRemove={remove}
              onMove={move}
              onLaunch={(target) => {
                ipc
                  .launchApp(target.id)
                  .then(() => onToast(`Launching ${target.name}…`))
                  .catch((e: unknown) => onToast(`Launch failed: ${String(e)}`));
              }}
            />
          ))}
        </ul>
      )}
    </div>
  );
}

function AppRow({
  app,
  index,
  total,
  icon,
  isMissing,
  onUpdate,
  onRemove,
  onMove,
  onLaunch,
}: {
  app: LauncherApp;
  index: number;
  total: number;
  icon: string | null;
  isMissing: boolean;
  onUpdate: (id: string, patch: Partial<LauncherApp>) => void;
  onRemove: (id: string) => void;
  onMove: (index: number, delta: number) => void;
  onLaunch: (app: LauncherApp) => void;
}) {
  const [expanded, setExpanded] = useState(false);
  const [argsText, setArgsText] = useState((app.arguments ?? []).join(" "));
  const errors = validateLauncherApp(app);

  return (
    <li className={"app-row" + (isMissing ? " is-missing" : "")}>
      <div className="app-row-main">
        <div className="app-row-icon">
          <span className="app-icon-preview">
            {icon ? <img src={icon} alt="" draggable={false} /> : null}
          </span>
        </div>
        <div className="app-row-fields">
          <input
            type="text"
            className="app-name"
            value={app.name}
            placeholder="Display name"
            onChange={(e) => onUpdate(app.id, { name: e.target.value })}
          />
          <div className={"app-path" + (isMissing ? " has-error" : "")} title={app.executablePath}>
            {app.executablePath}
            {isMissing ? <span className="missing-badge">file missing</span> : null}
          </div>
        </div>
        <div className="app-row-actions">
          <button
            type="button"
            className="btn btn-icon"
            title={isMissing ? "Executable missing" : "Launch now"}
            disabled={isMissing}
            onClick={() => onLaunch(app)}
          >
            ▶
          </button>
          <input
            type="checkbox"
            checked={app.enabled}
            title={app.enabled ? "Enabled" : "Disabled"}
            aria-label="Enabled"
            onChange={(e) => onUpdate(app.id, { enabled: e.target.checked })}
          />
          <button
            type="button"
            className="btn btn-icon"
            title="Move up"
            disabled={index === 0}
            onClick={() => onMove(index, -1)}
          >
            ↑
          </button>
          <button
            type="button"
            className="btn btn-icon"
            title="Move down"
            disabled={index === total - 1}
            onClick={() => onMove(index, 1)}
          >
            ↓
          </button>
          <button
            type="button"
            className="btn btn-icon"
            title="Details"
            aria-expanded={expanded}
            onClick={() => setExpanded((v) => !v)}
          >
            {expanded ? "▴" : "▾"}
          </button>
          <button
            type="button"
            className="btn btn-icon btn-danger"
            title="Remove"
            onClick={() => onRemove(app.id)}
          >
            ✕
          </button>
        </div>
      </div>

      {expanded ? (
        <div className="app-row-details">
          <TextField
            label="Arguments"
            value={argsText}
            placeholder='e.g. --profile "C:\My Folder"'
            onChange={(v) => {
              setArgsText(v);
              onUpdate(app.id, { arguments: splitCommandLine(v) });
            }}
            mono
          />
          <div className="inline-field">
            <TextField
              label="Custom icon"
              value={app.iconPath ?? ""}
              placeholder="Optional image file"
              onChange={(v) => onUpdate(app.id, { iconPath: v || undefined })}
              mono
            />
            <ApplicationPicker
              kind="icon"
              label="Choose…"
              onPicked={(p) => onUpdate(app.id, { iconPath: p })}
            />
            {app.iconPath ? (
              <button
                type="button"
                className="btn"
                onClick={() => onUpdate(app.id, { iconPath: undefined })}
              >
                Clear
              </button>
            ) : null}
          </div>
          <div className="inline-field">
            <TextField
              label="Working directory"
              value={app.workingDirectory ?? ""}
              placeholder="Optional"
              onChange={(v) => onUpdate(app.id, { workingDirectory: v || undefined })}
              mono
            />
            <ApplicationPicker
              kind="directory"
              label="Browse…"
              onPicked={(p) => onUpdate(app.id, { workingDirectory: p })}
            />
            {app.workingDirectory ? (
              <button
                type="button"
                className="btn"
                onClick={() => onUpdate(app.id, { workingDirectory: undefined })}
              >
                Clear
              </button>
            ) : null}
          </div>
          {errors.length > 0 ? (
            <div className="field-error">
              {errors.map((e) => (
                <div key={e}>{e}</div>
              ))}
            </div>
          ) : null}
        </div>
      ) : null}
    </li>
  );
}

export function BehaviorSection({ settings, onSettingsChange }: SectionProps) {
  return (
    <div className="section">
      <h2>Behavior</h2>
      <p className="section-desc">How selection and launching respond to input.</p>

      <Toggle
        label="Launch when the button is released"
        hint="When off, releasing keeps the menu open and you pick an item with a click."
        checked={settings.launchOnRelease}
        onChange={(v) => onSettingsChange({ launchOnRelease: v })}
      />

      <Toggle
        label="Close when no item is selected"
        hint="When off, the menu stays open after releasing over the dead zone; click an item or anywhere outside to dismiss it."
        checked={!settings.stickyWhenNoSelection}
        onChange={(v) => onSettingsChange({ stickyWhenNoSelection: !v })}
      />

      <Slider
        label="Selection sensitivity (boundary margin)"
        value={settings.hysteresisDeg}
        min={0}
        max={10}
        step={0.5}
        onChange={(v) => onSettingsChange({ hysteresisDeg: v })}
        format={(v) => `${v}°`}
      />
      <div className="field-hint">
        Extra angular margin the cursor must travel before switching to a neighboring item.
        Reduces flicker when the cursor rests near a sector boundary. 0 disables it.
      </div>

      <div className="callout">
        <strong>Keyboard:</strong> the settings window is fully keyboard accessible. A global
        keyboard shortcut for opening the menu is planned but not enabled in this release.
      </div>
    </div>
  );
}
