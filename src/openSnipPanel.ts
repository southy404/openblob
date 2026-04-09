import { WebviewWindow } from "@tauri-apps/api/webviewWindow";

export async function ensureSnipPanelWindow() {
  const existing = await WebviewWindow.getByLabel("snip-panel");
  if (existing) return existing;

  return new WebviewWindow("snip-panel", {
    url: "/snip-panel.html",
    title: "Snip Panel",
    transparent: false,
    decorations: true,
    alwaysOnTop: true,
    resizable: true,
    width: 520,
    height: 760,
    visible: false,
    center: true,
    skipTaskbar: false,
  });
}
