import React, { useEffect, useState } from "react";
import ReactDOM from "react-dom/client";
import { listen } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { invoke } from "@tauri-apps/api/core";

type TimerPayload = {
  minutes: number;
  seconds: number;
  label: string;
  startedAt: number;
};

type FinishedPayload = {
  minutes: number;
  seconds: number;
  text: string;
};

type TimerState = {
  secondsLeft: number;
  label: string;
} | null;

function formatTime(totalSeconds: number) {
  const mins = Math.floor(totalSeconds / 60);
  const secs = totalSeconds % 60;
  return `${mins.toString().padStart(2, "0")}:${secs
    .toString()
    .padStart(2, "0")}`;
}

function TimerOverlayApp() {
  const [timer, setTimer] = useState<TimerState>(null);

  useEffect(() => {
    const applyGlass = async () => {
      try {
        const win = getCurrentWindow();
        await invoke("apply_glass_effect", { window: win });
      } catch (error) {
        console.error("failed to apply glass effect", error);
      }
    };

    void applyGlass();
  }, []);

  useEffect(() => {
    let unlistenStarted: null | (() => void) = null;
    let unlistenFinished: null | (() => void) = null;
    let intervalId: number | null = null;

    const setup = async () => {
      unlistenStarted = await listen<TimerPayload>(
        "companion-timer-started",
        async (event) => {
          const payload = event.payload;

          setTimer({
            secondsLeft: payload.seconds,
            label: payload.label || "Timer",
          });

          await getCurrentWindow()
            .show()
            .catch(() => {});
        }
      );

      unlistenFinished = await listen<FinishedPayload>(
        "companion-timer-finished",
        async () => {
          setTimer(null);
          await getCurrentWindow()
            .hide()
            .catch(() => {});
        }
      );
    };

    void setup();

    intervalId = window.setInterval(() => {
      setTimer((prev) => {
        if (!prev) return prev;
        if (prev.secondsLeft <= 1) return null;
        return { ...prev, secondsLeft: prev.secondsLeft - 1 };
      });
    }, 1000);

    return () => {
      unlistenStarted?.();
      unlistenFinished?.();
      if (intervalId) window.clearInterval(intervalId);
    };
  }, []);

  return (
    <>
      <style>{`
      :root {
        color-scheme: dark;
        --text-main: #ffffff;
        --text-soft: rgba(255, 255, 255, 0.72);
        --text-dim: rgba(255, 255, 255, 0.5);
      }

      * {
        box-sizing: border-box;
      }

      html, body, #root {
        width: 100%;
        height: 100%;
        margin: 0;
        background: transparent;
        overflow: hidden;
        font-family: -apple-system, BlinkMacSystemFont, "SF Pro Display", "Segoe UI", sans-serif;
        color: var(--text-main);
      }

      .timer-shell {
        width: 100%;
        height: 100%;
        display: flex;
        align-items: flex-start;
        justify-content: flex-end;
        padding: 12px;
        pointer-events: none;
        background: transparent;
      }

      .timer-card {
        position: relative;
        min-width: 158px;
        max-width: 168px;
        border-radius: 22px;
        isolation: isolate;
        overflow: hidden;
        border: 1px solid rgba(255, 255, 255, 0.14);
        background: rgba(24, 24, 28, 0.28);
        backdrop-filter: blur(18px) saturate(155%);
        -webkit-backdrop-filter: blur(18px) saturate(155%);
        box-shadow:
          inset 0 1px 1px rgba(255, 255, 255, 0.18),
          inset 0 -1px 1px rgba(0, 0, 0, 0.16);
        backface-visibility: hidden;
        padding: 12px 14px;
      }

      .timer-card::before {
        content: "";
        position: absolute;
        inset: 0;
        pointer-events: none;
        border-radius: inherit;
        background:
          radial-gradient(circle at top left, rgba(255, 255, 255, 0.18), transparent 34%),
          radial-gradient(circle at bottom right, rgba(255, 255, 255, 0.05), transparent 36%);
        z-index: 0;
      }

      .timer-inner {
        position: relative;
        z-index: 1;
      }

      .timer-title {
        font-size: 10px;
        font-weight: 700;
        letter-spacing: 0.08em;
        text-transform: uppercase;
        color: var(--text-dim);
        margin-bottom: 6px;
      }

      .timer-time {
        font-size: 28px;
        line-height: 1;
        font-weight: 700;
        letter-spacing: -0.04em;
        color: var(--text-main);
        text-rendering: optimizeLegibility;
      }

      .timer-label {
        margin-top: 7px;
        font-size: 11px;
        line-height: 1.3;
        color: var(--text-soft);
        white-space: nowrap;
        overflow: hidden;
        text-overflow: ellipsis;
        max-width: 130px;
      }
    `}</style>

      <div className="timer-shell">
        {timer && (
          <div className="timer-card">
            <div className="timer-inner">
              <div className="timer-title">Timer</div>
              <div className="timer-time">{formatTime(timer.secondsLeft)}</div>
              <div className="timer-label">{timer.label}</div>
            </div>
          </div>
        )}
      </div>
    </>
  );
}

ReactDOM.createRoot(document.getElementById("root")!).render(
  <React.StrictMode>
    <TimerOverlayApp />
  </React.StrictMode>
);
