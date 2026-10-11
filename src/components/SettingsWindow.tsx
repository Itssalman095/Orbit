import { useEffect, useState } from "react";
import { onNavigate } from "../lib/events";
import { useConfig } from "../hooks/useConfig";
import { ipc } from "../lib/ipc";
import {
  AppearanceSection,
  ApplicationsSection,
  BehaviorSection,
  GeneralSection,
} from "../pages/Settings";
import { AboutPage } from "../pages/About";

export type TabId = "general" | "appearance" | "applications" | "behavior" | "about";

const TABS: { id: TabId; label: string }[] = [
  { id: "general", label: "General" },
  { id: "appearance", label: "Appearance" },
  { id: "applications", label: "Applications" },
  { id: "behavior", label: "Behavior" },
  { id: "about", label: "About" },
];

/**
 * Shell of the settings window: sidebar navigation, config state, toasts.
 * The window itself is a separate OS window from the radial overlay.
 */
export function SettingsWindow() {
  const cfg = useConfig();
  const [tab, setTab] = useState<TabId>("general");

  useEffect(() => {
    const un = onNavigate((t) => {
      if (TABS.some((x) => x.id === t)) setTab(t as TabId);
    });
    return () => {
      un.then((f) => f());
    };
  }, []);

  if (cfg.error && !cfg.config) {
    return (
      <div className="settings-root">
        <div className="fatal-error">
          <h2>Could not load configuration</h2>
          <p>{cfg.error}</p>
          <button type="button" className="btn" onClick={cfg.reload}>
            Retry
          </button>
        </div>
      </div>
    );
  }

  if (!cfg.config) {
    return (
      <div className="settings-root">
        <div className="boot-splash">Loading…</div>
      </div>
    );
  }

  const { settings, applications } = cfg.config;
  const sectionProps = {
    settings,
    onSettingsChange: cfg.setSettings,
  };

  return (
    <div className="settings-root" data-theme={cfg.config.settings.theme}>
      <aside className="sidebar">
        <div className="brand">
          <div className="brand-mark" aria-hidden="true">
            <span />
          </div>
          <div className="brand-name">Orbit</div>
        </div>
        <nav className="nav" aria-label="Settings sections">
          {TABS.map((t) => (
            <button
              key={t.id}
              type="button"
              className={"nav-item" + (tab === t.id ? " is-active" : "")}
              onClick={() => setTab(t.id)}
            >
              {t.label}
            </button>
          ))}
        </nav>
        <div className="sidebar-footer">
          <div className={"status-pill" + (settings.enabled ? " is-on" : "")}>
            {settings.enabled ? "Launcher active" : "Launcher disabled"}
          </div>
          <button type="button" className="btn btn-quit" onClick={() => void ipc.quitApp()}>
            Quit Orbit
          </button>
        </div>
      </aside>

      <main className="content">
        <header className="content-header">
          <div className="save-state" aria-live="polite">
            {cfg.saving ? "Saving…" : "All changes saved"}
          </div>
        </header>
        {cfg.error ? <div className="error-banner">Save failed: {cfg.error}</div> : null}
        <div className="content-body">
          {tab === "general" ? (
            <GeneralSection {...sectionProps} onOpenAbout={() => setTab("about")} />
          ) : null}
          {tab === "appearance" ? (
            <AppearanceSection
              {...sectionProps}
              applications={applications}
              onToast={cfg.toast}
            />
          ) : null}
          {tab === "applications" ? (
            <ApplicationsSection
              applications={applications}
              missing={cfg.missing}
              onApplicationsChange={cfg.setApplications}
              onToast={cfg.toast}
            />
          ) : null}
          {tab === "behavior" ? <BehaviorSection {...sectionProps} /> : null}
          {tab === "about" ? <AboutPage /> : null}
        </div>
      </main>

      <div className="toasts" aria-live="polite">
        {cfg.toasts.map((t) => (
          <div key={t.id} className="toast">
            {t.message}
          </div>
        ))}
      </div>
    </div>
  );
}
