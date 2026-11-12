import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";

export type Icon = "desktop" | "laptop" | "plug";

export interface PortView {
  value: number;
  portLabel: string;
  name: string | null;
  displayName: string;
  isLocal: boolean;
  isCurrent: boolean;
  hotkey: string | null;
  icon: Icon;
}

export interface MonitorView {
  key: string;
  label: string;
  connection: "hdmi" | "displayPort" | "usbTunnel" | "other";
  current: number | null;
  currentName: string | null;
  error: string | null;
  switchError: string | null;
  ports: PortView[];
  togglePair: [number, number] | null;
  togglePairIsDefault: boolean;
  localOverride: number | null;
  readCapabilities: boolean;
}

/** What the update reminder shows. */
export type UpdateStatus =
  | { state: "idle" }
  | { state: "checking" }
  | { state: "upToDate" }
  | { state: "available"; version: string; notes: string | null }
  | { state: "downloading"; version: string; percent: number | null }
  /** `version` is set when installing it failed, null when checking failed. */
  | { state: "failed"; message: string; version: string | null };

export interface View {
  monitors: MonitorView[];
  showOsd: boolean;
  autostart: boolean;
  lanDiscovery: boolean;
  toggleHotkey: string | null;
  computerName: string;
  failedHotkeys: string[];
  checkUpdates: boolean;
  appVersion: string;
  update: UpdateStatus;
}

export type OptionName = "showOsd" | "autostart" | "lanDiscovery" | "checkUpdates";

export const api = {
  getView: () => invoke<View>("get_view"),
  refresh: () => invoke<void>("refresh"),
  switchInput: (key: string, value: number) => invoke<void>("switch_input", { key, value }),
  toggle: () => invoke<void>("toggle"),
  setPortName: (key: string, value: number, name: string | null) =>
    invoke<void>("set_port_name", { key, value, name }),
  setPortHotkey: (key: string, value: number, accelerator: string | null) =>
    invoke<void>("set_port_hotkey", { key, value, accelerator }),
  setToggleHotkey: (accelerator: string | null) => invoke<void>("set_toggle_hotkey", { accelerator }),
  startHotkeyRecording: () => invoke<void>("start_hotkey_recording"),
  stopHotkeyRecording: () => invoke<void>("stop_hotkey_recording"),
  setTogglePair: (key: string, pair: [number, number] | null) => invoke<void>("set_toggle_pair", { key, pair }),
  setLocalOverride: (key: string, value: number | null) => invoke<void>("set_local_override", { key, value }),
  setReadCapabilities: (key: string, enabled: boolean) => invoke<void>("set_read_capabilities", { key, enabled }),
  setOption: (name: OptionName, value: boolean) => invoke<void>("set_option", { name, value }),
  openSettings: () => invoke<void>("open_settings"),
  hideFlyout: () => invoke<void>("hide_flyout"),
  fitFlyout: (height: number) => invoke<void>("fit_flyout", { height }),
  checkForUpdates: () => invoke<void>("check_for_updates"),
  installUpdate: () => invoke<void>("install_update"),
  openReleasePage: () => invoke<void>("open_release_page"),
};

export function onView(callback: (view: View) => void): Promise<UnlistenFn> {
  return listen<View>("view-changed", (e) => callback(e.payload));
}

/** A combination caught by the system-wide recorder (Windows), e.g. "Ctrl+Alt+M". */
export function onHotkeyRecorded(callback: (accelerator: string) => void): Promise<UnlistenFn> {
  return listen<string>("hotkey-recorded", (e) => callback(e.payload));
}

const ENTITIES: Record<string, string> = { "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;", "'": "&#39;" };

/** Escape text before putting it into innerHTML. Names come from the user. */
export function esc(text: string): string {
  return text.replace(/[&<>"']/g, (c) => ENTITIES[c]);
}
