import { LogicalPosition } from "@tauri-apps/api/window";
import { WebviewWindow } from "@tauri-apps/api/webviewWindow";

const BUBBLE_WIDTH = 1040;
const BUBBLE_HEIGHT = 135;

// 0 = direkt auf Workarea-Unterkante
// Falls du 2-4 px Luft willst, hier leicht erhöhen
const BOTTOM_MARGIN = 0;

function getWorkArea() {
  const screenAny = window.screen as Screen & {
    availLeft?: number;
    availTop?: number;
  };

  return {
    left: screenAny.availLeft ?? 0,
    top: screenAny.availTop ?? 0,
    width: window.screen.availWidth,
    height: window.screen.availHeight,
  };
}

export async function positionBubbleWindow(win?: WebviewWindow) {
  const bubble = win ?? (await WebviewWindow.getByLabel("bubble"));
  if (!bubble) return;

  try {
    const workArea = getWorkArea();

    const x = Math.round(workArea.left + (workArea.width - BUBBLE_WIDTH) / 2);
    const y = Math.round(
      workArea.top + workArea.height - BUBBLE_HEIGHT - BOTTOM_MARGIN
    );

    await bubble.setPosition(new LogicalPosition(x, y));
  } catch (error) {
    console.error("failed to position bubble window", error);
  }
}

export async function ensureBubbleWindow() {
  const existing = await WebviewWindow.getByLabel("bubble");
  if (existing) {
    await positionBubbleWindow(existing);
    return existing;
  }

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
    focus: true,
  });

  win.once("tauri://created", async () => {
    await positionBubbleWindow(win);

    // second pass for Windows/Tauri
    window.setTimeout(() => {
      void positionBubbleWindow(win);
    }, 80);
  });

  return win;
}
