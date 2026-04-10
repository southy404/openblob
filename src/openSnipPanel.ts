import { WebviewWindow } from "@tauri-apps/api/webviewWindow";

export async function ensureSnipPanelWindow() {
  const existing = await WebviewWindow.getByLabel("snip-panel");
  if (existing) return existing;

  return new WebviewWindow("snip-panel", {
    url: "/snip-panel.html",
    title: "Snip Panel",
    transparent: true,
    decorations: false,
    alwaysOnTop: true,
    shadow: false,
    resizable: true,
    width: 620,
    height: 820,
    visible: false,
    center: true,
    skipTaskbar: false,
  });
}
