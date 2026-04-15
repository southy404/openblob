import { LogicalPosition } from "@tauri-apps/api/window";
import { WebviewWindow } from "@tauri-apps/api/webviewWindow";

function sleep(ms: number) {
  return new Promise((resolve) => setTimeout(resolve, ms));
}

const QUICK_MENU_LABEL = "quick-menu";
const QUICK_MENU_WIDTH = 360;
const QUICK_MENU_HEIGHT = 640;
const QUICK_MENU_PADDING = 14;

export async function ensureQuickMenuWindow() {
  const existing = await WebviewWindow.getByLabel(QUICK_MENU_LABEL);
  if (existing) return existing;

  const win = new WebviewWindow(QUICK_MENU_LABEL, {
    url: "quick-menu.html",
    title: "Quick Menu",
    transparent: true,
    decorations: false,
    alwaysOnTop: true,
    shadow: false,
    resizable: false,
    width: QUICK_MENU_WIDTH,
    height: QUICK_MENU_HEIGHT,
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

  await sleep(100);
  return win;
}

export async function showQuickMenuWindow(x: number, y: number) {
  const win = await ensureQuickMenuWindow();

  const screenWidth = window.screen.availWidth || window.screen.width;
  const screenHeight = window.screen.availHeight || window.screen.height;

  const finalX = Math.max(
    QUICK_MENU_PADDING,
    Math.min(Math.round(x), screenWidth - QUICK_MENU_WIDTH - QUICK_MENU_PADDING)
  );

  const finalY = Math.max(
    QUICK_MENU_PADDING,
    Math.min(
      Math.round(y),
      screenHeight - QUICK_MENU_HEIGHT - QUICK_MENU_PADDING
    )
  );

  await win.setPosition(new LogicalPosition(finalX, finalY)).catch(() => {});
  await win.show().catch(() => {});
  await win.setFocus().catch(() => {});

  await sleep(70);
  return win;
}

export async function hideQuickMenuWindow() {
  const win = await WebviewWindow.getByLabel(QUICK_MENU_LABEL);
  if (!win) return;
  await win.hide().catch(() => {});
}
