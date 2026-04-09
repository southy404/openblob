import React, { useEffect, useMemo, useRef, useState } from "react";
import ReactDOM from "react-dom/client";
import { invoke } from "@tauri-apps/api/core";
import { emit } from "@tauri-apps/api/event";
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

function SnipOverlay() {
  const [start, setStart] = useState<Point | null>(null);
  const [end, setEnd] = useState<Point | null>(null);
  const [dragging, setDragging] = useState(false);
  const [busy, setBusy] = useState(false);
  const rootRef = useRef<HTMLDivElement | null>(null);

  const rect = useMemo(() => {
    if (!start || !end) return null;
    return makeRect(start, end);
  }, [start, end]);

  useEffect(() => {
    const onKey = async (e: KeyboardEvent) => {
      if (e.key === "Escape") {
        await getCurrentWindow().close();
      }
      if (
        e.key === "Enter" &&
        rect &&
        rect.width > 2 &&
        rect.height > 2 &&
        !busy
      ) {
        await confirmCapture(rect);
      }
    };

    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [rect, busy]);

  async function confirmCapture(r: Rect) {
    setBusy(true);

    try {
      console.log("[snip] confirmCapture rect:", r);

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

      await getCurrentWindow().close();
    } catch (err) {
      console.error("[snip] capture failed", err);
      alert(`Snip capture failed: ${String(err)}`);
      setBusy(false);
    }
  }

  function onMouseDown(e: React.MouseEvent<HTMLDivElement>) {
    if (busy) return;
    const p = { x: e.clientX, y: e.clientY };
    setStart(p);
    setEnd(p);
    setDragging(true);
  }

  function onMouseMove(e: React.MouseEvent<HTMLDivElement>) {
    if (!dragging || !start) return;
    setEnd({ x: e.clientX, y: e.clientY });
  }

  async function onMouseUp() {
    setDragging(false);
    if (rect && rect.width > 2 && rect.height > 2) {
      await confirmCapture(rect);
    }
  }

  return (
    <div
      ref={rootRef}
      className="snip-overlay-root"
      onMouseDown={onMouseDown}
      onMouseMove={onMouseMove}
      onMouseUp={onMouseUp}
    >
      <div className="snip-hint">Drag to snip • Enter confirm • Esc cancel</div>

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
