import { LogicalPosition } from "@tauri-apps/api/window";
import { WebviewWindow } from "@tauri-apps/api/webviewWindow";

function sleep(ms: number) {
  return new Promise((resolve) => setTimeout(resolve, ms));
}

export async function ensureTimerOverlayWindow() {
  const existing = await WebviewWindow.getByLabel("timer-overlay");
  if (existing) return existing;

  const win = new WebviewWindow("timer-overlay", {
    url: "timer-overlay.html",
    title: "Timer Overlay",
    transparent: true,
    decorations: false,
    alwaysOnTop: true,
    shadow: false,
    resizable: false,
    width: 168,
    height: 78,
    visible: false,
    skipTaskbar: true,
    focus: false,
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

  await sleep(80);
  return win;
}

export async function showTimerOverlayWindow() {
  const win = await ensureTimerOverlayWindow();

  const overlayWidth = 168;
  const overlayHeight = 78;
  const marginRight = 16;
  const marginBottom = 0;

  const screenWidth = window.screen.availWidth || window.screen.width;
  const screenHeight = window.screen.availHeight || window.screen.height;

  const x = Math.round(screenWidth - overlayWidth - marginRight);
  const y = Math.round(screenHeight - overlayHeight - marginBottom);

  await win.setPosition(new LogicalPosition(x, y)).catch(() => {});
  await win.show().catch(() => {});
}

export async function hideTimerOverlayWindow() {
  const win = await WebviewWindow.getByLabel("timer-overlay");
  if (!win) return;
  await win.hide().catch(() => {});
}
