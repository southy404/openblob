import { LogicalPosition } from "@tauri-apps/api/window";
import { WebviewWindow } from "@tauri-apps/api/webviewWindow";

const BUBBLE_WIDTH = 1040;
const BUBBLE_HEIGHT = 320;
const BOTTOM_MARGIN = 22;

export async function ensureBubbleWindow() {
  const existing = await WebviewWindow.getByLabel("bubble");
  if (existing) return existing;

  const win = new WebviewWindow("bubble", {
    url: "bubble.html",
    title: "Bubble",
    transparent: true,
    decorations: false,
    alwaysOnTop: true,
    shadow: false,
    skipTaskbar: true,
    resizable: false,
    width: BUBBLE_WIDTH,
    height: BUBBLE_HEIGHT,
    visible: false,
  });

  win.once("tauri://created", async () => {
    try {
      const screenWidth = window.screen.availWidth;
      const screenHeight = window.screen.availHeight;

      const x = Math.round((screenWidth - BUBBLE_WIDTH) / 2);
      const y = Math.round(screenHeight - BUBBLE_HEIGHT - BOTTOM_MARGIN);

      await win.setPosition(new LogicalPosition(x, y));
    } catch (error) {
      console.error("failed to position bubble window", error);
    }
  });

  return win;
}
