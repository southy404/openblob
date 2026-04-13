import { emitTo } from "@tauri-apps/api/event";
import { WebviewWindow } from "@tauri-apps/api/webviewWindow";

function sleep(ms: number) {
  return new Promise((resolve) => setTimeout(resolve, ms));
}

export async function openSnipOverlay() {
  let overlay = await WebviewWindow.getByLabel("snip-overlay");

  if (!overlay) {
    overlay = new WebviewWindow("snip-overlay", {
      url: "snip-overlay.html",
      title: "Snip Overlay",
      transparent: true,
      decorations: false,
      alwaysOnTop: true,
      shadow: false,
      skipTaskbar: true,
      resizable: false,
      fullscreen: true,
      visible: false,
      focus: true,
    });

    await new Promise<void>((resolve, reject) => {
      let settled = false;

      overlay!.once("tauri://created", () => {
        if (settled) return;
        settled = true;
        resolve();
      });

      overlay!.once("tauri://error", (e) => {
        if (settled) return;
        settled = true;
        reject(e);
      });
    });

    // kleiner zusätzlicher Tick für den ersten Start
    await sleep(120);
  }

  await overlay.show().catch(() => {});
  await overlay.setFocus().catch(() => {});

  // dem Webview Zeit geben, Listener / Rendering sauber zu haben
  await sleep(140);

  await emitTo("snip-overlay", "snip-overlay-open");
}
