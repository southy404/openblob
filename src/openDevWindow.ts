import { WebviewWindow } from "@tauri-apps/api/webviewWindow";

export async function ensureDevWindow() {
  const existing = await WebviewWindow.getByLabel("bubble-dev");
  if (existing) return existing;

  return new WebviewWindow("bubble-dev", {
    url: "bubble-dev.html",
    title: "OpenBlob Dev Mode",
    transparent: true,
    decorations: false,
    alwaysOnTop: true,
    shadow: false,
    skipTaskbar: false,
    resizable: true,
    width: 760,
    height: 560,
    visible: false,
    center: true,
  });
}
