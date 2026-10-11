import { describe, expect, it } from "vitest";
import {
  DEFAULT_SETTINGS,
  nameFromPath,
  sanitizeApp,
  sanitizeConfig,
  splitCommandLine,
  validateLauncherApp,
} from "./configValidation";
import type { LauncherApp } from "../types/config";

function makeApp(overrides: Partial<LauncherApp> = {}): LauncherApp {
  return {
    id: "app-1",
    name: "Test App",
    executablePath: "C:\\Program Files\\Test\\app.exe",
    enabled: true,
    ...overrides,
  };
}

describe("sanitizeConfig", () => {
  it("returns defaults for garbage input", () => {
    const cfg = sanitizeConfig(undefined);
    expect(cfg.version).toBe(1);
    expect(cfg.settings).toEqual(DEFAULT_SETTINGS);
    expect(cfg.applications).toEqual([]);
  });

  it("returns defaults for non-object input", () => {
    expect(sanitizeConfig(42).settings.radius).toBe(DEFAULT_SETTINGS.radius);
    expect(sanitizeConfig("nope").settings.enabled).toBe(true);
  });

  it("clamps out-of-range numbers", () => {
    const cfg = sanitizeConfig({
      settings: { radius: 99999, iconSize: 1, deadZoneRadius: -50, menuOpacity: 3, animationSpeed: -5 },
    });
    expect(cfg.settings.radius).toBe(400);
    expect(cfg.settings.iconSize).toBe(24);
    expect(cfg.settings.deadZoneRadius).toBe(0);
    expect(cfg.settings.menuOpacity).toBe(1);
    expect(cfg.settings.animationSpeed).toBe(0);
  });

  it("falls back when fields have the wrong type", () => {
    const cfg = sanitizeConfig({
      settings: { enabled: "yes", trigger: "banana", radius: "140", theme: 7 },
    });
    expect(cfg.settings.enabled).toBe(true); // non-boolean -> default
    expect(cfg.settings.trigger).toBe("middle");
    expect(cfg.settings.radius).toBe(140); // numeric strings are accepted
    expect(cfg.settings.theme).toBe("dark");
  });

  it("drops applications with missing ids or executable paths", () => {
    const cfg = sanitizeConfig({
      applications: [
        { id: "ok", name: "Ok", executablePath: "C:\\ok.exe", enabled: true },
        { name: "No id", executablePath: "C:\\x.exe", enabled: true },
        { id: "no-path", name: "No path", enabled: true },
        "not even an object",
        null,
      ],
    });
    expect(cfg.applications).toHaveLength(1);
    expect(cfg.applications[0]?.id).toBe("ok");
  });

  it("deduplicates application ids preserving order", () => {
    const cfg = sanitizeConfig({
      applications: [
        { id: "dup", name: "First", executablePath: "C:\\first.exe", enabled: true },
        { id: "dup", name: "Second", executablePath: "C:\\second.exe", enabled: true },
      ],
    });
    expect(cfg.applications).toHaveLength(1);
    expect(cfg.applications[0]?.name).toBe("First");
  });

  it("fills a missing name from the executable path", () => {
    const cfg = sanitizeConfig({
      applications: [{ id: "a", name: "", executablePath: "C:\\Tools\\MyTool.EXE", enabled: true }],
    });
    expect(cfg.applications[0]?.name).toBe("MyTool");
  });

  it("sanitizes arguments arrays, dropping non-string entries", () => {
    const cfg = sanitizeConfig({
      applications: [
        {
          id: "a",
          name: "A",
          executablePath: "C:\\a.exe",
          enabled: true,
          arguments: ["--flag", 42, "  ", "ok"],
        },
      ],
    });
    expect(cfg.applications[0]?.arguments).toEqual(["--flag", "ok"]);
  });
});

describe("sanitizeApp", () => {
  it("rejects non-objects", () => {
    expect(sanitizeApp(null)).toBeNull();
    expect(sanitizeApp("x")).toBeNull();
  });

  it("trims strings", () => {
    const app = sanitizeApp({ id: " a ", name: " Name ", executablePath: " C:\\x.exe " });
    expect(app?.id).toBe("a");
    expect(app?.name).toBe("Name");
    expect(app?.executablePath).toBe("C:\\x.exe");
  });

  it("defaults enabled to true", () => {
    const app = sanitizeApp({ id: "a", name: "A", executablePath: "C:\\a.exe" });
    expect(app?.enabled).toBe(true);
  });
});

describe("validateLauncherApp", () => {
  it("accepts a well-formed app", () => {
    expect(validateLauncherApp(makeApp())).toEqual([]);
  });

  it("flags missing executable paths", () => {
    const errors = validateLauncherApp(makeApp({ executablePath: "" }));
    expect(errors.some((e) => e.includes("required"))).toBe(true);
  });

  it("warns about non-executable targets", () => {
    const errors = validateLauncherApp(makeApp({ executablePath: "C:\\x\\doc.txt" }));
    expect(errors.some((e) => e.includes(".exe"))).toBe(true);
  });

  it("flags missing names", () => {
    const errors = validateLauncherApp(makeApp({ name: "" }));
    expect(errors.some((e) => e.includes("name"))).toBe(true);
  });
});

describe("nameFromPath", () => {
  it("extracts the file base name", () => {
    expect(nameFromPath("C:\\Program Files\\Notepad++\\notepad++.exe")).toBe("notepad++");
    expect(nameFromPath("C:\\Apps\\My App.lnk")).toBe("My App");
  });

  it("handles trailing separators and edge cases", () => {
    expect(nameFromPath("C:\\dir\\")).toBe("dir");
  });
});

describe("splitCommandLine", () => {
  it("splits on whitespace", () => {
    expect(splitCommandLine("--a --b value")).toEqual(["--a", "--b", "value"]);
  });

  it("honors double quotes", () => {
    expect(splitCommandLine('--profile "C:\\My Dir" --verbose')).toEqual([
      "--profile",
      "C:\\My Dir",
      "--verbose",
    ]);
  });

  it("returns an empty array for blank input", () => {
    expect(splitCommandLine("   ")).toEqual([]);
  });
});
