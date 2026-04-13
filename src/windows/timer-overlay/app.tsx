import React, { useEffect, useState } from "react";
import ReactDOM from "react-dom/client";
import { listen } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";

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
    let unlistenStarted: null | (() => void) = null;
    let unlistenFinished: null | (() => void) = null;
    let intervalId: number | null = null;

    const setup = async () => {
      unlistenStarted = await listen<TimerPayload>(
        "companion-timer-started",
        async (event) => {
          const payload = event.payload;

          console.log("[timer-overlay] started", payload);

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
        async (event) => {
          console.log("[timer-overlay] finished", event.payload);

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
          font-family: Inter, ui-sans-serif, system-ui, -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif;
        }

        .timer-shell {
          width: 100%;
          height: 100%;
          display: flex;
          align-items: flex-start;
          justify-content: flex-end;
          padding: 8px;
          pointer-events: none;
          background: transparent;
        }

        .timer-card {
          min-width: 148px;
          border-radius: 18px;
          border: 1px solid rgba(255,255,255,0.14);
          background:
            linear-gradient(
              180deg,
              rgba(255,255,255,0.16),
              rgba(255,255,255,0.06)
            ),
            rgba(16,20,28,0.72);
          backdrop-filter: blur(18px) saturate(145%);
          -webkit-backdrop-filter: blur(18px) saturate(145%);
          padding: 10px 12px;
          color: rgba(255,255,255,0.96);
          box-shadow: 0 10px 30px rgba(0,0,0,0.18);
        }

        .timer-title {
          font-size: 10px;
          font-weight: 800;
          opacity: 0.68;
          letter-spacing: 0.08em;
          margin-bottom: 4px;
        }

        .timer-time {
          font-size: 24px;
          font-weight: 800;
          line-height: 1;
        }

        .timer-label {
          margin-top: 6px;
          font-size: 11px;
          opacity: 0.62;
        }
      `}</style>

      <div className="timer-shell">
        {timer && (
          <div className="timer-card">
            <div className="timer-title">TIMER</div>
            <div className="timer-time">{formatTime(timer.secondsLeft)}</div>
            <div className="timer-label">{timer.label}</div>
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
