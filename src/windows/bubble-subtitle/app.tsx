import { createRoot } from "react-dom/client";
import { listen } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { useEffect, useRef, useState } from "react";

type SubtitlePayload = {
  text: string;
  holdMs?: number;
};

function SubtitleApp() {
  const [displayedAnswer, setDisplayedAnswer] = useState("");
  const [visible, setVisible] = useState(false);

  const revealTimerRef = useRef<number | null>(null);
  const fadeTimerRef = useRef<number | null>(null);

  const clearRevealTimer = () => {
    if (revealTimerRef.current !== null) {
      window.clearInterval(revealTimerRef.current);
      revealTimerRef.current = null;
    }
  };

  const clearFadeTimer = () => {
    if (fadeTimerRef.current !== null) {
      window.clearTimeout(fadeTimerRef.current);
      fadeTimerRef.current = null;
    }
  };

  const clearSubtitle = async () => {
    clearRevealTimer();
    clearFadeTimer();
    setDisplayedAnswer("");
    setVisible(false);

    try {
      await getCurrentWindow().hide();
    } catch (error) {
      console.error("[subtitle] hide failed", error);
    }
  };

  const revealAnswerWordByWord = async (text: string, holdMs = 5200) => {
    clearRevealTimer();
    clearFadeTimer();

    const trimmed = text.trim();
    setDisplayedAnswer("");

    if (!trimmed) {
      await clearSubtitle();
      return;
    }

    try {
      await getCurrentWindow().show();
    } catch (error) {
      console.error("[subtitle] show failed", error);
    }

    const words = trimmed.split(/\s+/);
    let index = 0;

    setVisible(true);

    revealTimerRef.current = window.setInterval(() => {
      index += 1;
      setDisplayedAnswer(words.slice(0, index).join(" "));

      if (index >= words.length) {
        clearRevealTimer();

        fadeTimerRef.current = window.setTimeout(() => {
          setVisible(false);

          window.setTimeout(() => {
            void getCurrentWindow()
              .hide()
              .catch((error) => {
                console.error("[subtitle] delayed hide failed", error);
              });
          }, 240);
        }, holdMs);
      }
    }, 34);
  };

  useEffect(() => {
    let unlistenShow: null | (() => void) = null;
    let unlistenClear: null | (() => void) = null;

    const setup = async () => {
      console.log("[subtitle] app loaded");

      unlistenShow = await listen<SubtitlePayload>(
        "bubble-subtitle-show",
        async (event) => {
          console.log("[subtitle] show event", event.payload);
          await revealAnswerWordByWord(
            event.payload.text,
            event.payload.holdMs ?? 5200
          );
        }
      );

      unlistenClear = await listen("bubble-subtitle-clear", async () => {
        console.log("[subtitle] clear event");
        await clearSubtitle();
      });
    };

    void setup();

    return () => {
      unlistenShow?.();
      unlistenClear?.();
      clearRevealTimer();
      clearFadeTimer();
    };
  }, []);

  return (
    <>
      <style>{`
        html,
        body,
        #root {
          width: 100%;
          height: 100%;
          margin: 0;
          background: transparent;
          overflow: hidden;
          font-family: -apple-system, BlinkMacSystemFont, "SF Pro Display", "Segoe UI", sans-serif;
        }

        * {
          box-sizing: border-box;
        }

        .subtitle-stage {
          width: 100%;
          height: 100%;
          display: flex;
          align-items: flex-end;
          justify-content: center;
          pointer-events: none;
          background: transparent;
          padding: 0 12px;
        }

        .subtitle-text {
          width: 100%;
          text-align: center;
          color: #ffffff;
          font-size: clamp(24px, 2.15vw, 38px);
          line-height: 1.24;
          font-weight: 700;
          letter-spacing: 0.01em;
          white-space: pre-wrap;
          word-break: break-word;
          text-shadow:
            0 4px 16px rgba(0, 0, 0, 0.4),
            0 1px 2px rgba(0, 0, 0, 0.8);
          user-select: none;
          transition: opacity 320ms ease, transform 320ms ease;
        }
      `}</style>

      <div className="subtitle-stage">
        <div
          className="subtitle-text"
          style={{
            opacity: visible ? 1 : 0,
            transform: visible
              ? "translateY(0px) scale(1)"
              : "translateY(8px) scale(0.995)",
          }}
        >
          {displayedAnswer}
        </div>
      </div>
    </>
  );
}

createRoot(document.getElementById("root")!).render(<SubtitleApp />);
