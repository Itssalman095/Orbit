/**
 * React state management for the configuration shown in the settings window.
 * The Rust backend is the source of truth; this hook mirrors it with
 * debounced saves so the UI stays responsive while sliders are dragged.
 */

import { useCallback, useEffect, useRef, useState } from "react";
import { ipc } from "../lib/ipc";
import { onConfigReloaded, onLaunchError } from "../lib/events";
import type { Config, LauncherApp, Settings } from "../types/config";

export interface UseConfigResult {
  config: Config | null;
  /** app id -> executable missing on disk */
  missing: Record<string, boolean>;
  error: string | null;
  saving: boolean;
  setSettings: (patch: Partial<Settings>) => void;
  setApplications: (apps: LauncherApp[]) => void;
  reload: () => void;
  toast: (message: string) => void;
  toasts: { id: number; message: string }[];
}

export function useConfig(): UseConfigResult {
  const [config, setConfig] = useState<Config | null>(null);
  const [missing, setMissing] = useState<Record<string, boolean>>({});
  const [error, setError] = useState<string | null>(null);
  const [saving, setSaving] = useState(false);
  const [toasts, setToasts] = useState<{ id: number; message: string }[]>([]);
  const saveTimer = useRef<ReturnType<typeof setTimeout> | null>(null);
  const pendingRef = useRef<Config | null>(null);
  const toastId = useRef(0);

  const toast = useCallback((message: string) => {
    const id = ++toastId.current;
    setToasts((t) => [...t, { id, message }]);
    setTimeout(() => setToasts((t) => t.filter((x) => x.id !== id)), 5000);
  }, []);

  const applyResponse = useCallback((data: { config: Config; missing: Record<string, boolean> }) => {
    setConfig(data.config);
    setMissing(data.missing ?? {});
  }, []);

  const reload = useCallback(() => {
    ipc
      .loadConfig()
      .then(applyResponse)
      .catch((e) => setError(String(e)));
  }, [applyResponse]);

  useEffect(() => {
    reload();
    let unlistenReload: Unlisten = () => {};
    let unlistenErr: Unlisten = () => {};
    onConfigReloaded(() => {
      reload();
    }).then((u) => (unlistenReload = u));
    onLaunchError((payload) => {
      toast(`Failed to launch ${payload.appName}: ${payload.message}`);
    }).then((u) => (unlistenErr = u));
    return () => {
      unlistenReload();
      unlistenErr();
    };
  }, [reload, toast]);

  // Debounced save of the latest config.
  useEffect(() => {
    if (pendingRef.current === null) return;
    if (saveTimer.current !== null) clearTimeout(saveTimer.current);
    saveTimer.current = setTimeout(() => {
      const snapshot = pendingRef.current;
      pendingRef.current = null;
      if (!snapshot) return;
      setSaving(true);
      ipc
        .saveConfig(snapshot)
        .then(applyResponse)
        .catch((e) => {
          setError(String(e));
          toast("Could not save configuration.");
        })
        .finally(() => setSaving(false));
    }, 350);
    return () => {
      if (saveTimer.current !== null) clearTimeout(saveTimer.current);
    };
  }, [config, applyResponse, toast]);

  const setSettings = useCallback(
    (patch: Partial<Settings>) => {
      setConfig((prev) => {
        if (!prev) return prev;
        const next = { ...prev, settings: { ...prev.settings, ...patch } };
        pendingRef.current = next;
        return next;
      });
    },
    [],
  );

  const setApplications = useCallback((apps: LauncherApp[]) => {
    setConfig((prev) => {
      if (!prev) return prev;
      const next = { ...prev, applications: apps };
      pendingRef.current = next;
      return next;
    });
  }, []);

  return {
    config,
    missing,
    error,
    saving,
    setSettings,
    setApplications,
    reload,
    toast,
    toasts,
  };
}

type Unlisten = () => void;
