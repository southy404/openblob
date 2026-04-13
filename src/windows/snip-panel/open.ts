import { WebviewWindow } from "@tauri-apps/api/webviewWindow";

function sleep(ms: number) {
  return new Promise((resolve) => setTimeout(resolve, ms));
}

export async function ensureSnipPanelWindow() {
  const existing = await WebviewWindow.getByLabel("snip-panel");
  if (existing) return existing;

  const win = new WebviewWindow("snip-panel", {
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

  await sleep(120);

  return win;
}
