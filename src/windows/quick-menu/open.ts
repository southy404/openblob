import { LogicalPosition } from "@tauri-apps/api/window";
import { WebviewWindow } from "@tauri-apps/api/webviewWindow";

function sleep(ms: number) {
  return new Promise((resolve) => setTimeout(resolve, ms));
}

export async function ensureQuickMenuWindow() {
  const existing = await WebviewWindow.getByLabel("quick-menu");
  if (existing) return existing;

  const win = new WebviewWindow("quick-menu", {
    url: "/quick-menu.html",
    title: "Quick Menu",
    transparent: true,
    decorations: false,
    alwaysOnTop: true,
    shadow: false,
    resizable: false,
    width: 360,
    height: 640,
    visible: false,
    center: false,
    skipTaskbar: true,
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

export async function showQuickMenuWindow(x: number, y: number) {
  const win = await ensureQuickMenuWindow();

  const menuWidth = 360;
  const menuHeight = 640;
  const padding = 14;

  const screenWidth = window.screen.availWidth || window.screen.width;
  const screenHeight = window.screen.availHeight || window.screen.height;

  const finalX = Math.max(
    padding,
    Math.min(Math.round(x), screenWidth - menuWidth - padding)
  );

  const finalY = Math.max(
    padding,
    Math.min(Math.round(y), screenHeight - menuHeight - padding)
  );

  await win.setPosition(new LogicalPosition(finalX, finalY)).catch(() => {});
  await win.show().catch(() => {});
  await win.setFocus().catch(() => {});

  await sleep(90);
  return win;
}

export async function hideQuickMenuWindow() {
  const win = await WebviewWindow.getByLabel("quick-menu");
  if (!win) return;
  await win.hide().catch(() => {});
}
