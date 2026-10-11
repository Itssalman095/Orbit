import { useEffect, useState } from "react";
import { getVersion } from "@tauri-apps/api/app";

export function AboutPage() {
  const [version, setVersion] = useState("1.0.0");

  useEffect(() => {
    getVersion()
      .then(setVersion)
      .catch(() => setVersion("1.0.0"));
  }, []);

  return (
    <div className="section">
      <h2>Orbit</h2>
      <p className="section-desc">
        A radial application launcher for Windows. Version {version}.
      </p>

      <h3>How to use</h3>
      <ol className="about-list">
        <li>Keep Orbit running — it lives in the system tray.</li>
        <li>
          Press and hold the <span className="kbd">middle mouse button</span> anywhere in Windows.
        </li>
        <li>Move toward an application and release the button to launch it.</li>
        <li>Release over the center (or outside the ring) to dismiss the menu.</li>
      </ol>

      <h3>Under the hood</h3>
      <ul className="about-list">
        <li>Native low-level mouse hook (WH_MOUSE_LL) in Rust — no simulated browser events.</li>
        <li>Transparent, click-through, non-activating overlay positioned at the physical cursor.</li>
        <li>Per-Monitor V2 DPI awareness; coordinates are handled in physical pixels.</li>
        <li>Applications are launched through the Win32 API with validated, stored paths only.</li>
        <li>Configuration is stored as JSON in your AppData directory.</li>
      </ul>

      <h3>Troubleshooting</h3>
      <ul className="about-list">
        <li>
          If the menu stops appearing, some heavy utility may have installed a conflicting mouse
          hook — restart Orbit from the tray menu (Reload configuration) or restart Windows.
        </li>
        <li>
          While the launcher is enabled, the activation button is captured globally; disable the
          launcher from the tray or Settings to give the button back to other apps.
        </li>
        <li>Missing applications are reported when launched and flagged in Settings.</li>
      </ul>

      <p className="about-footer">
        Built with Tauri 2, React, TypeScript and Rust. Released under the MIT License.
      </p>
    </div>
  );
}
