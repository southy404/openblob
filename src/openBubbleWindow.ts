import { WebviewWindow } from "@tauri-apps/api/webviewWindow";

export async function ensureBubbleWindow() {
  const existing = await WebviewWindow.getByLabel("bubble");
  if (existing) return existing;

  return new WebviewWindow("bubble", {
    url: "bubble.html",
    title: "Companion Bubble",
    transparent: true,
    decorations: false,
    alwaysOnTop: true,
    shadow: false,
    skipTaskbar: true,
    resizable: true,
    width: 1120,
    height: 340,
    visible: true,
    x: 380,
    y: 720,
  });
}
