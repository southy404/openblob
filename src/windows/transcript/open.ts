import { WebviewWindow } from "@tauri-apps/api/webviewWindow";

function sleep(ms: number) {
  return new Promise((resolve) => setTimeout(resolve, ms));
}

const TRANSCRIPT_LABEL = "transcript";
const TRANSCRIPT_WIDTH = 1180;
const TRANSCRIPT_HEIGHT = 860;

export async function ensureTranscriptWindow() {
  const existing = await WebviewWindow.getByLabel(TRANSCRIPT_LABEL);
  if (existing) return existing;

  const win = new WebviewWindow(TRANSCRIPT_LABEL, {
    url: "transcript.html",
    title: "Transcript",
    transparent: true,
    decorations: false,
    alwaysOnTop: false,
    shadow: false,
    resizable: true,
    width: TRANSCRIPT_WIDTH,
    height: TRANSCRIPT_HEIGHT,
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

export async function showTranscriptWindow() {
  const win = await ensureTranscriptWindow();
  await win.show().catch(() => {});
  await win.setFocus().catch(() => {});
  return win;
}
