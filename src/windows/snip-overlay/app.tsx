import React, { useEffect, useMemo, useRef, useState } from "react";
import ReactDOM from "react-dom/client";
import { invoke } from "@tauri-apps/api/core";
import { emit, listen } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";
import "./snip-overlay.css";

type Point = { x: number; y: number };
type Rect = { x: number; y: number; width: number; height: number };

function makeRect(a: Point, b: Point): Rect {
  const x = Math.min(a.x, b.x);
  const y = Math.min(a.y, b.y);
  const width = Math.abs(a.x - b.x);
  const height = Math.abs(a.y - b.y);
  return { x, y, width, height };
}

function sleep(ms: number) {
  return new Promise((resolve) => setTimeout(resolve, ms));
}

function SnipOverlay() {
  const [start, setStart] = useState<Point | null>(null);
  const [end, setEnd] = useState<Point | null>(null);
  const [dragging, setDragging] = useState(false);
  const [busy, setBusy] = useState(false);

  const rootRef = useRef<HTMLDivElement | null>(null);
  const busyRef = useRef(false);

  useEffect(() => {
    busyRef.current = busy;
  }, [busy]);

  const rect = useMemo(() => {
    if (!start || !end) return null;
    return makeRect(start, end);
  }, [start, end]);

  const resetOverlayState = () => {
    setStart(null);
    setEnd(null);
    setDragging(false);
    setBusy(false);
    busyRef.current = false;
  };

  useEffect(() => {
    let unlistenOpen: null | (() => void) = null;

    const setup = async () => {
      unlistenOpen = await listen("snip-overlay-open", async () => {
        console.log("[snip-overlay] open event received");

        resetOverlayState();

        const win = getCurrentWindow();
        await win.show().catch(() => {});
        await win.setFocus().catch(() => {});

        requestAnimationFrame(() => {
          rootRef.current?.focus();
        });
      });
    };

    void setup();

    return () => {
      unlistenOpen?.();
    };
  }, []);

  useEffect(() => {
    const onKey = async (e: KeyboardEvent) => {
      if (e.key === "Escape") {
        resetOverlayState();
        await getCurrentWindow()
          .hide()
          .catch(() => {});
        return;
      }

      if (
        e.key === "Enter" &&
        rect &&
        rect.width > 2 &&
        rect.height > 2 &&
        !busyRef.current
      ) {
        await confirmCapture(rect);
      }
    };

    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [rect]);

  async function confirmCapture(r: Rect) {
    if (busyRef.current) return;

    busyRef.current = true;
    setBusy(true);

    try {
      console.log("[snip] confirmCapture rect:", r);

      const win = getCurrentWindow();

      // Overlay vor Capture verstecken
      await win.hide().catch(() => {});

      // Einen Frame + kleinen Delay geben, damit das Hide wirklich durch ist
      await new Promise<void>((resolve) => {
        requestAnimationFrame(() => resolve());
      });
      await sleep(120);

      const path = await invoke<string>("capture_snip_region", {
        x: Math.round(r.x),
        y: Math.round(r.y),
        width: Math.round(r.width),
        height: Math.round(r.height),
      });

      console.log("[snip] capture path:", path);

      await emit("snip-created", {
        path,
        rect: {
          x: Math.round(r.x),
          y: Math.round(r.y),
          width: Math.round(r.width),
          height: Math.round(r.height),
        },
      });

      console.log("[snip] emitted snip-created");

      resetOverlayState();
    } catch (err) {
      console.error("[snip] capture failed", err);
      alert(`Snip capture failed: ${String(err)}`);

      const win = getCurrentWindow();
      await win.show().catch(() => {});
      await win.setFocus().catch(() => {});

      busyRef.current = false;
      setBusy(false);
    }
  }

  function onMouseDown(e: React.MouseEvent<HTMLDivElement>) {
    if (busyRef.current) return;

    const p = { x: e.clientX, y: e.clientY };
    setStart(p);
    setEnd(p);
    setDragging(true);
  }

  function onMouseMove(e: React.MouseEvent<HTMLDivElement>) {
    if (!dragging || !start || busyRef.current) return;
    setEnd({ x: e.clientX, y: e.clientY });
  }

  async function onMouseUp(e: React.MouseEvent<HTMLDivElement>) {
    if (!dragging || !start || busyRef.current) {
      setDragging(false);
      return;
    }

    const finalPoint = { x: e.clientX, y: e.clientY };
    const finalRect = makeRect(start, finalPoint);

    setEnd(finalPoint);
    setDragging(false);

    if (finalRect.width > 2 && finalRect.height > 2) {
      await confirmCapture(finalRect);
    }
  }

  return (
    <div
      ref={rootRef}
      className="snip-overlay-root"
      tabIndex={-1}
      onMouseDown={onMouseDown}
      onMouseMove={onMouseMove}
      onMouseUp={onMouseUp}
    >
      <div className="snip-hint">
        {busy ? "Capturing..." : "Drag to snip • Enter confirm • Esc cancel"}
      </div>

      {rect && (
        <div
          className="snip-selection"
          style={{
            left: rect.x,
            top: rect.y,
            width: rect.width,
            height: rect.height,
          }}
        />
      )}
    </div>
  );
}

ReactDOM.createRoot(document.getElementById("root")!).render(
  <React.StrictMode>
    <SnipOverlay />
  </React.StrictMode>
);
