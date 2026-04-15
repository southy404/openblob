import { LogicalPosition } from "@tauri-apps/api/window";
import { WebviewWindow } from "@tauri-apps/api/webviewWindow";

const SUBTITLE_WIDTH = 1180;
const SUBTITLE_HEIGHT = 190;
const SUBTITLE_BOTTOM_OFFSET = 150;

export async function ensureSubtitleWindow() {
  const existing = await WebviewWindow.getByLabel("bubble-subtitle");
  if (existing) return existing;

  const win = new WebviewWindow("bubble-subtitle", {
    url: "bubble-subtitle.html",
    title: "Bubble Subtitle",
    transparent: true,
    decorations: false,
    alwaysOnTop: true,
    shadow: false,
    skipTaskbar: true,
    resizable: false,
    width: SUBTITLE_WIDTH,
    height: SUBTITLE_HEIGHT,
    visible: false,
    focus: false,
  });

  win.once("tauri://created", async () => {
    try {
      const screenWidth = window.screen.availWidth;
      const screenHeight = window.screen.availHeight;

      const x = Math.round((screenWidth - SUBTITLE_WIDTH) / 2);
      const y = Math.round(
        screenHeight - SUBTITLE_HEIGHT - SUBTITLE_BOTTOM_OFFSET
      );

      await win.setPosition(new LogicalPosition(x, y));
    } catch (error) {
      console.error(
        "[subtitle-window] failed to position subtitle window",
        error
      );
    }
  });

  return win;
}
