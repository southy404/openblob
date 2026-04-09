import { WebviewWindow } from "@tauri-apps/api/webviewWindow";

export async function openSnipOverlay() {
  const existing = await WebviewWindow.getByLabel("snip-overlay");
  if (existing) {
    await existing.setFocus();
    return existing;
  }

  const overlay = new WebviewWindow("snip-overlay", {
    url: "/snip-overlay.html",
    title: "Snip Overlay",
    transparent: true,
    decorations: false,
    alwaysOnTop: true,
    skipTaskbar: true,
    resizable: false,
    maximizable: false,
    minimizable: false,
    closable: true,
    focus: true,
    fullscreen: true,
  });

  return overlay;
}
