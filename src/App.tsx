import { getCurrentWebview } from "@tauri-apps/api/webview";
import { Overlay } from "./overlay/Overlay";
import { SettingsWindow } from "./components/SettingsWindow";

/**
 * Entry component. The same webview bundle serves two windows:
 *  - "settings": the full settings UI
 *  - "overlay":  the transparent radial menu (renderer only)
 */
export default function App() {
  const label = getCurrentWebview().label;
  return label === "overlay" ? <Overlay /> : <SettingsWindow />;
}
