import { WebviewWindow } from "@tauri-apps/api/webviewWindow";

export async function ensureBubbleWindow() {
  const existing = await WebviewWindow.getByLabel("bubble");
  if (existing) return existing;

  return new WebviewWindow("bubble", {
    url: "bubble.html",
    title: "OpenBlob Bubble",
    transparent: true,
    decorations: false,
    alwaysOnTop: true,
    shadow: false,
    skipTaskbar: true,
    resizable: false,
    width: 1320,
    height: 430,
    visible: false,
    focus: false,
    x: 280,
    y: 560,
  });
}
