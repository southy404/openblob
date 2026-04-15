import { WebviewWindow } from "@tauri-apps/api/webviewWindow";

function sleep(ms: number) {
  return new Promise((resolve) => setTimeout(resolve, ms));
}

const SNIP_PANEL_LABEL = "snip-panel";
const SNIP_PANEL_WIDTH = 620;
const SNIP_PANEL_HEIGHT = 820;

export async function ensureSnipPanelWindow() {
  const existing = await WebviewWindow.getByLabel(SNIP_PANEL_LABEL);
  if (existing) return existing;

  const win = new WebviewWindow(SNIP_PANEL_LABEL, {
    url: "snip-panel.html",
    title: "Snip Panel",
    transparent: true,
    decorations: false,
    alwaysOnTop: true,
    shadow: false,
    resizable: true,
    width: SNIP_PANEL_WIDTH,
    height: SNIP_PANEL_HEIGHT,
    visible: false,
    center: true,
    skipTaskbar: false,
    focus: true,
  });

  await new Promise<void>((resolve, reject) => {
    let settled = false;

    win.once("tauri://created", () => {
      if (settled) return;
      settled = true;
      resolve();
    });

    win.once("tauri://error", (e) => {
      if (settled) return;
      settled = true;
      reject(e);
    });
  });

  await sleep(100);
  return win;
}
