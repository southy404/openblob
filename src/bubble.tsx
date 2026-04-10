import { useEffect, useMemo, useRef, useState } from "react";
import { createRoot } from "react-dom/client";
import { invoke } from "@tauri-apps/api/core";
import { listen, emit, emitTo } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { Mic, MicOff, Send, Volume2, VolumeX } from "lucide-react";
import { ensureDevWindow } from "./openDevWindow";

type ContextPayload = {
  text: string;
  hint?: string;
  autoRun?: boolean;
};

type OllamaResult = {
  content: string;
  model: string;
};

type SpeechRecognitionLike = {
  lang: string;
  interimResults: boolean;
  continuous: boolean;
  maxAlternatives: number;
  onstart: null | (() => void);
  onend: null | (() => void);
  onerror: null | ((event: { error?: string }) => void);
  onresult: null | ((event: SpeechRecognitionEventLike) => void);
  start: () => void;
  stop: () => void;
};

type SpeechRecognitionEventLike = {
  resultIndex: number;
  results: ArrayLike<{
    isFinal?: boolean;
    0: { transcript: string };
  }>;
};

declare global {
  interface Window {
    webkitSpeechRecognition?: new () => SpeechRecognitionLike;
    SpeechRecognition?: new () => SpeechRecognitionLike;
  }
}

const STORAGE_KEYS = {
  model: "openblob-bubble-model",
  voiceShortcut: "openblob-bubble-voice-shortcut",
  speakEnabled: "openblob-bubble-speak-enabled",
};

function readLocalStorageString(key: string, fallback: string) {
  try {
    const value = window.localStorage.getItem(key);
    return value ?? fallback;
  } catch {
    return fallback;
  }
}

function readLocalStorageBool(key: string, fallback: boolean) {
  try {
    const value = window.localStorage.getItem(key);
    if (value === null) return fallback;
    return value === "true";
  } catch {
    return fallback;
  }
}

function normalizeShortcutLabel(input: string) {
  return input
    .replace(/\s+/g, " ")
    .replace(/control/gi, "Ctrl")
    .replace(/escape/gi, "Esc")
    .replace(/command/gi, "Cmd")
    .trim();
}

function speak(text: string) {
  if (!("speechSynthesis" in window) || !text.trim()) return;
  window.speechSynthesis.cancel();

  const utter = new SpeechSynthesisUtterance(text);
  utter.rate = 1;
  utter.pitch = 1;
  utter.lang = "de-DE";
  window.speechSynthesis.speak(utter);
}

function stopSpeaking() {
  if ("speechSynthesis" in window) {
    window.speechSynthesis.cancel();
  }
}

function isHideAndSeekCommand(input: string) {
  const q = input.trim().toLowerCase();
  return (
    q.includes("hide and seek") ||
    q.includes("lets play hide and seek") ||
    q.includes("let's play hide and seek") ||
    q.includes("lass uns verstecken spielen") ||
    q.includes("verstecken spielen")
  );
}

function looksLikeDirectCommand(input: string) {
  const q = input.trim().toLowerCase();

  return (
    q.startsWith("open ") ||
    q.startsWith("öffne ") ||
    q.startsWith("oeffne ") ||
    q.startsWith("start ") ||
    q.startsWith("starte ") ||
    q.startsWith("launch ") ||
    q.startsWith("run ") ||
    q.startsWith("mute") ||
    q.startsWith("unmute") ||
    q.startsWith("save") ||
    q.startsWith("new tab") ||
    q.startsWith("close tab") ||
    q.startsWith("reload") ||
    q.startsWith("google ") ||
    q.startsWith("youtube ")
  );
}

function getDirectKnownUrl(input: string): string | null {
  const q = input.trim().toLowerCase();

  switch (q) {
    case "open youtube":
    case "oeffne youtube":
    case "start youtube":
    case "starte youtube":
    case "launch youtube":
    case "run youtube":
      return "https://www.youtube.com";

    case "open netflix":
    case "oeffne netflix":
    case "start netflix":
    case "starte netflix":
    case "launch netflix":
    case "run netflix":
      return "https://www.netflix.com";

    case "open spotify":
    case "oeffne spotify":
    case "start spotify":
    case "starte spotify":
    case "launch spotify":
    case "run spotify":
      return "https://open.spotify.com";

    case "open twitch":
    case "oeffne twitch":
    case "start twitch":
    case "starte twitch":
    case "launch twitch":
    case "run twitch":
      return "https://www.twitch.tv";

    case "open github":
    case "oeffne github":
    case "start github":
    case "starte github":
    case "launch github":
    case "run github":
      return "https://github.com";

    case "open google":
    case "oeffne google":
    case "start google":
    case "starte google":
    case "launch google":
    case "run google":
      return "https://www.google.com";

    default:
      return null;
  }
}

function BubbleApp() {
  const [question, setQuestion] = useState("");
  const [answer, setAnswer] = useState("");
  const [displayedAnswer, setDisplayedAnswer] = useState("");
  const [hint, setHint] = useState("Bereit.");
  const [model, setModel] = useState(
    readLocalStorageString(STORAGE_KEYS.model, "llama3.1:8b")
  );
  const [busy, setBusy] = useState(false);
  const [visible, setVisible] = useState(false);
  const [listening, setListening] = useState(false);
  const [interimText, setInterimText] = useState("");
  const [lastRoute, setLastRoute] = useState<"command" | "ollama" | "none">(
    "none"
  );
  const [voiceShortcut, setVoiceShortcut] = useState(
    readLocalStorageString(STORAGE_KEYS.voiceShortcut, "Alt + M")
  );
  const [speakEnabled, setSpeakEnabled] = useState(
    readLocalStorageBool(STORAGE_KEYS.speakEnabled, true)
  );
  const [subtitleVisible, setSubtitleVisible] = useState(false);

  const inputRef = useRef<HTMLInputElement | null>(null);
  const recognitionRef = useRef<SpeechRecognitionLike | null>(null);
  const revealTimerRef = useRef<number | null>(null);
  const subtitleFadeTimerRef = useRef<number | null>(null);

  const SpeechRecognitionCtor = useMemo(
    () => window.SpeechRecognition || window.webkitSpeechRecognition || null,
    []
  );

  const clearRevealTimer = () => {
    if (revealTimerRef.current !== null) {
      window.clearInterval(revealTimerRef.current);
      revealTimerRef.current = null;
    }
  };

  const clearSubtitleFadeTimer = () => {
    if (subtitleFadeTimerRef.current !== null) {
      window.clearTimeout(subtitleFadeTimerRef.current);
      subtitleFadeTimerRef.current = null;
    }
  };

  const focusInputSoon = () => {
    window.setTimeout(() => {
      inputRef.current?.focus();
      const len = inputRef.current?.value.length ?? 0;
      inputRef.current?.setSelectionRange(len, len);
    }, 110);
  };

  const fadeInAndShow = async () => {
    const win = getCurrentWindow();
    await win.show();
    requestAnimationFrame(() => setVisible(true));
    await win.setFocus().catch(() => {});
    focusInputSoon();
  };

  const fadeOutAndHide = async () => {
    setVisible(false);
    window.setTimeout(async () => {
      await getCurrentWindow()
        .hide()
        .catch(() => {});
    }, 180);
  };

  const closeBubble = async () => {
    stopVoiceInput();
    await fadeOutAndHide();
  };

  const emitBlobState = async (
    state: "thinking" | "listening",
    active: boolean
  ) => {
    try {
      await emit("blob-state", { state, active });
    } catch {}
  };

  const revealAnswerWordByWord = (text: string, holdMs = 4200) => {
    clearRevealTimer();
    clearSubtitleFadeTimer();

    const trimmed = text.trim();
    setAnswer(trimmed);
    setDisplayedAnswer("");

    if (!trimmed) {
      setSubtitleVisible(false);
      return;
    }

    const words = trimmed.split(/\s+/);
    let index = 0;

    setSubtitleVisible(true);

    revealTimerRef.current = window.setInterval(() => {
      index += 1;
      setDisplayedAnswer(words.slice(0, index).join(" "));

      if (index >= words.length) {
        clearRevealTimer();

        subtitleFadeTimerRef.current = window.setTimeout(() => {
          setSubtitleVisible(false);
        }, holdMs);
      }
    }, 34);
  };

  const showSubtitle = (text: string, holdMs = 5200) => {
    revealAnswerWordByWord(text, holdMs);
  };

  const openDevWindow = async () => {
    const dev = await ensureDevWindow();
    await dev.show();
    await dev.setFocus().catch(() => {});
    await emitTo("bubble-dev", "bubble-dev-data", {
      lastRoute,
      voiceShortcut,
      model,
    });
  };

  const runOllamaAsk = async (prompt: string) => {
    await emitBlobState("thinking", true);

    try {
      const result = await invoke<OllamaResult>("ask_ollama", {
        mode: "ask",
        text: prompt,
        question: prompt,
        model,
      });

      showSubtitle(result.content, 5600);
      setHint(`Antwort von ${result.model}`);
      setLastRoute("ollama");

      await emit("companion-speech", result.content.slice(0, 180));

      if (speakEnabled) {
        speak(result.content.slice(0, 260));
      }
    } finally {
      await emitBlobState("thinking", false);
    }
  };

  const executeCommandOrAsk = async (rawInput: string) => {
    const input = rawInput.trim();

    if (!input || busy) {
      setHint("Bitte gib etwas ein.");
      return;
    }

    setQuestion(input);
    setBusy(true);

    const directUrl = getDirectKnownUrl(input);

    try {
      if (directUrl) {
        await invoke<string>("handle_voice_command", {
          input: `open ${directUrl}`,
        });

        showSubtitle(`Öffne ${directUrl}.`, 4200);
        setHint("Bekannte Seite direkt geöffnet.");
        setLastRoute("command");

        if (speakEnabled) {
          speak(`Öffne ${directUrl}.`);
        }
        return;
      }

      if (isHideAndSeekCommand(input)) {
        await emit("start-hide-and-seek");
        showSubtitle("Okay, hide and seek started. Find me.", 4200);
        setHint("Hide and seek started.");
        setLastRoute("command");

        if (speakEnabled) {
          speak("Okay, hide and seek started. Find me.");
        }
        return;
      }

      const actionResult = await invoke<string>("handle_voice_command", {
        input,
      });

      if (actionResult !== "NO_ACTION") {
        showSubtitle(actionResult, 4200);
        setHint("Befehl ausgeführt.");
        setLastRoute("command");

        await emit("companion-speech", actionResult);

        if (speakEnabled) {
          speak(actionResult.slice(0, 220));
        }

        return;
      }

      if (looksLikeDirectCommand(input)) {
        const message = `Konnte den lokalen Befehl nicht ausführen: "${input}"`;
        showSubtitle(message, 4200);
        setHint("Befehl erkannt, aber lokal nichts Passendes gefunden.");
        setLastRoute("command");

        if (speakEnabled) {
          speak(message);
        }
        return;
      }

      setHint("Kein lokaler Befehl erkannt. Frage Ollama...");
      await runOllamaAsk(input);
    } catch (error) {
      const message = String(error);

      showSubtitle(message, 4800);

      if (looksLikeDirectCommand(input)) {
        setHint("Lokaler Befehl fehlgeschlagen.");
        setLastRoute("command");
      } else {
        setHint(`Fehler: ${message}`);
      }

      if (speakEnabled) {
        speak(message.slice(0, 220));
      }
    } finally {
      setBusy(false);
    }
  };

  const handleTypedSubmit = async () => {
    if (busy) return;
    await executeCommandOrAsk(question);
  };

  const startVoiceInput = async () => {
    if (!SpeechRecognitionCtor) {
      setHint("SpeechRecognition wird hier nicht unterstützt.");
      return;
    }

    if (listening || busy) return;

    try {
      const recognition = new SpeechRecognitionCtor();
      recognition.lang = "de-DE";
      recognition.interimResults = true;
      recognition.continuous = false;
      recognition.maxAlternatives = 1;

      recognition.onstart = async () => {
        setListening(true);
        setInterimText("");
        setHint("Ich höre zu …");
        await emitBlobState("listening", true);
      };

      recognition.onresult = (event) => {
        let finalTranscript = "";
        let liveTranscript = "";

        for (let i = event.resultIndex; i < event.results.length; i++) {
          const chunk = event.results[i][0]?.transcript ?? "";
          if (event.results[i].isFinal) {
            finalTranscript += chunk;
          } else {
            liveTranscript += chunk;
          }
        }

        setInterimText(liveTranscript);

        if (finalTranscript.trim()) {
          setQuestion(finalTranscript.trim());
        }
      };

      recognition.onend = async () => {
        setListening(false);
        recognitionRef.current = null;
        setInterimText("");
        await emitBlobState("listening", false);

        const finalText = (inputRef.current?.value ?? question).trim();
        if (finalText) {
          await executeCommandOrAsk(finalText);
        }
      };

      recognition.onerror = async (event) => {
        setListening(false);
        recognitionRef.current = null;
        setInterimText("");
        setHint(`Voice error: ${event.error ?? "unbekannt"}`);
        await emitBlobState("listening", false);
      };

      recognitionRef.current = recognition;
      recognition.start();
    } catch (error) {
      setListening(false);
      setInterimText("");
      setHint(`Mikrofonfehler: ${String(error)}`);
      await emitBlobState("listening", false);
    }
  };

  const stopVoiceInput = () => {
    try {
      recognitionRef.current?.stop();
    } catch {}
    recognitionRef.current = null;
    setListening(false);
    setInterimText("");
    void emitBlobState("listening", false);
  };

  useEffect(() => {
    try {
      window.localStorage.setItem(STORAGE_KEYS.model, model);
    } catch {}
  }, [model]);

  useEffect(() => {
    try {
      window.localStorage.setItem(
        STORAGE_KEYS.voiceShortcut,
        normalizeShortcutLabel(voiceShortcut)
      );
    } catch {}
  }, [voiceShortcut]);

  useEffect(() => {
    try {
      window.localStorage.setItem(
        STORAGE_KEYS.speakEnabled,
        String(speakEnabled)
      );
    } catch {}
  }, [speakEnabled]);

  useEffect(() => {
    let unlistenContext: null | (() => void) = null;
    let unlistenToggle: null | (() => void) = null;
    let unlistenShow: null | (() => void) = null;
    let unlistenHide: null | (() => void) = null;
    let unlistenVoiceToggle: null | (() => void) = null;

    const setup = async () => {
      unlistenContext = await listen<ContextPayload>(
        "companion-context",
        async (event) => {
          const payload = event.payload;

          if (payload.text?.trim()) {
            showSubtitle(payload.text.trim(), 5200);
          }

          if (payload.hint) {
            setHint(payload.hint);
          }

          if (payload.autoRun && payload.text?.trim()) {
            setQuestion(payload.text.trim());
          }

          await fadeInAndShow();
        }
      );

      unlistenToggle = await listen("bubble-toggle", async () => {
        const win = getCurrentWindow();
        const isVisible = await win.isVisible();

        if (isVisible && visible) {
          stopVoiceInput();
          await fadeOutAndHide();
        } else {
          await fadeInAndShow();
        }
      });

      unlistenShow = await listen("bubble-show", async () => {
        await fadeInAndShow();
      });

      unlistenHide = await listen("bubble-hide", async () => {
        stopVoiceInput();
        await fadeOutAndHide();
      });

      unlistenVoiceToggle = await listen("companion-voice-toggle", async () => {
        await fadeInAndShow();

        if (listening) {
          stopVoiceInput();
        } else {
          await startVoiceInput();
        }
      });
    };

    void setup();

    return () => {
      if (unlistenContext) unlistenContext();
      if (unlistenToggle) unlistenToggle();
      if (unlistenShow) unlistenShow();
      if (unlistenHide) unlistenHide();
      if (unlistenVoiceToggle) unlistenVoiceToggle();
    };
  }, [visible, listening, question, busy]);

  useEffect(() => {
    const onKeyDown = (event: KeyboardEvent) => {
      const combo = [
        event.ctrlKey ? "Ctrl" : "",
        event.altKey ? "Alt" : "",
        event.shiftKey ? "Shift" : "",
        event.metaKey ? "Cmd" : "",
        event.key.length === 1 ? event.key.toUpperCase() : event.key,
      ]
        .filter(Boolean)
        .join(" + ");

      if (
        normalizeShortcutLabel(combo).toLowerCase() ===
        normalizeShortcutLabel(voiceShortcut).toLowerCase()
      ) {
        event.preventDefault();

        if (listening) stopVoiceInput();
        else void startVoiceInput();
      }
    };

    window.addEventListener("keydown", onKeyDown);
    return () => window.removeEventListener("keydown", onKeyDown);
  }, [listening, voiceShortcut, busy]);

  useEffect(() => {
    return () => {
      clearRevealTimer();
      clearSubtitleFadeTimer();
      stopSpeaking();

      try {
        recognitionRef.current?.stop();
      } catch {}
    };
  }, []);

  return (
    <>
      <style>{`
        :root {
          color-scheme: dark;
          --glass-base: rgba(16, 20, 28, 0.84);
          --glass-tint: rgba(255, 255, 255, 0.16);
          --glass-tint-soft: rgba(255, 255, 255, 0.09);
          --glass-border: rgba(255, 255, 255, 0.18);
          --glass-border-soft: rgba(255, 255, 255, 0.10);

          --text-main: rgba(255, 255, 255, 0.98);
          --text-soft: rgba(236, 240, 248, 0.82);
          --text-dim: rgba(236, 240, 248, 0.56);

          --subtitle-stroke: rgba(0, 0, 0, 0.92);
          --button-bg: rgba(255, 255, 255, 0.10);
          --button-bg-hover: rgba(255, 255, 255, 0.16);
        }

        html,
        body,
        #root {
          width: 100%;
          height: 100%;
          margin: 0;
          background: transparent;
          overflow: hidden;
          font-family: Inter, ui-sans-serif, system-ui, -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif;
          color: var(--text-main);
        }

        * {
          box-sizing: border-box;
        }

        .bubble-stage {
          width: 100%;
          height: 100%;
          padding: 24px 18px 26px;
          display: flex;
          flex-direction: column;
          justify-content: space-between;
          align-items: center;
          background: transparent;
        }

        .subtitle-stage {
          width: min(1180px, calc(100vw - 56px));
          min-height: 170px;
          display: flex;
          align-items: flex-end;
          justify-content: center;
          pointer-events: none;
          padding-top: 8px;
        }

        .subtitle-text {
          width: 100%;
          text-align: center;
          color: #fff;
          font-size: clamp(24px, 2.15vw, 38px);
          line-height: 1.24;
          font-weight: 800;
          letter-spacing: 0.01em;
          white-space: pre-wrap;
          word-break: break-word;
          text-shadow:
            0 1px 0 var(--subtitle-stroke),
            0 2px 0 var(--subtitle-stroke),
            0 3px 0 var(--subtitle-stroke),
            0 0 18px rgba(0, 0, 0, 0.32);
        }

        .subtitle-placeholder {
          color: rgba(255, 255, 255, 0);
        }

        .bottom-stack {
          width: min(1040px, calc(100vw - 28px));
          display: flex;
          flex-direction: column;
          align-items: center;
          gap: 9px;
        }

        .bubble-shell {
          width: 100%;
          position: relative;
          border-radius: 999px;
          overflow: hidden;
          isolation: isolate;
          background:
            linear-gradient(
              180deg,
              rgba(255,255,255,0.18),
              rgba(255,255,255,0.08)
            ),
            var(--glass-base);
          border: 1px solid var(--glass-border);
          backdrop-filter: blur(24px) saturate(145%);
          -webkit-backdrop-filter: blur(24px) saturate(145%);
        }

        .bubble-shell::before {
          content: "";
          position: absolute;
          inset: 0;
          border-radius: inherit;
          background:
            radial-gradient(circle at 18% 0%, rgba(255,255,255,0.18), transparent 34%),
            radial-gradient(circle at 80% 100%, rgba(255,255,255,0.08), transparent 30%);
          pointer-events: none;
          z-index: 0;
        }

        .bubble-shell::after {
          content: "";
          position: absolute;
          inset: 0;
          border-radius: inherit;
          pointer-events: none;
          z-index: 0;
          box-shadow:
            inset 1px 1px 0 rgba(255,255,255,0.28),
            inset -1px -1px 0 rgba(255,255,255,0.05);
        }

        .bubble-row {
          position: relative;
          z-index: 1;
          display: grid;
          grid-template-columns: 1fr auto auto auto;
          align-items: center;
          gap: 10px;
          min-height: 82px;
          padding: 11px 12px;
        }

        .input-wrap {
          min-width: 0;
          display: flex;
          flex-direction: column;
          justify-content: center;
          gap: 7px;
        }

        .bubble-input {
          width: 100%;
          height: 48px;
          border: 0;
          outline: none;
          background: transparent;
          color: var(--text-main);
          font-size: 16px;
          font-weight: 540;
          padding: 0 10px;
          text-shadow: 0 1px 0 rgba(0, 0, 0, 0.08);
        }

        .bubble-input::placeholder {
          color: var(--text-dim);
        }

        .bubble-meta {
          display: flex;
          gap: 10px;
          align-items: center;
          flex-wrap: wrap;
          padding-left: 10px;
          min-height: 16px;
        }

        .bubble-hint {
          font-size: 11px;
          color: var(--text-soft);
          white-space: nowrap;
          overflow: hidden;
          text-overflow: ellipsis;
          max-width: 100%;
        }

        .bubble-live {
          font-size: 11px;
          color: rgba(206, 223, 255, 0.92);
          white-space: nowrap;
          overflow: hidden;
          text-overflow: ellipsis;
          max-width: 340px;
        }

        .icon-btn {
          width: 52px;
          height: 52px;
          border-radius: 999px;
          border: 1px solid var(--glass-border-soft);
          background: var(--button-bg);
          color: var(--text-main);
          display: grid;
          place-items: center;
          cursor: pointer;
          transition:
            transform 0.14s ease,
            background 0.14s ease,
            border-color 0.14s ease,
            opacity 0.14s ease;
        }

        .icon-btn:hover {
          background: var(--button-bg-hover);
          border-color: rgba(255,255,255,0.18);
        }

        .icon-btn:active {
          transform: scale(0.98);
        }

        .icon-btn:disabled {
          opacity: 0.55;
          cursor: default;
        }

        .icon-btn-active {
          background: rgba(255,255,255,0.18);
          border-color: rgba(255,255,255,0.20);
        }

        .tiny-links {
          width: 100%;
          display: flex;
          justify-content: center;
          gap: 14px;
          min-height: 16px;
          flex-wrap: wrap;
        }

        .tiny-link {
          appearance: none;
          border: 0;
          background: transparent;
          padding: 0;
          font-size: 11px;
          color: rgba(255,255,255,0.68);
          cursor: pointer;
        }

        .tiny-link:hover {
          color: rgba(255,255,255,0.94);
        }

        .tiny-link-static {
          cursor: default;
        }

        @media (max-width: 820px) {
          .subtitle-stage {
            width: calc(100vw - 24px);
            min-height: 130px;
          }

          .subtitle-text {
            font-size: clamp(18px, 4vw, 28px);
          }

          .bottom-stack {
            width: calc(100vw - 16px);
          }

          .bubble-row {
            grid-template-columns: 1fr auto auto;
          }

          .sound-btn {
            display: none;
          }
        }
      `}</style>

      <div
        className="bubble-stage"
        style={{
          opacity: visible ? 1 : 0,
          transform: visible
            ? "translateY(0px) scale(1)"
            : "translateY(14px) scale(0.992)",
          transition: "opacity 180ms ease, transform 180ms ease",
          pointerEvents: visible ? "auto" : "none",
        }}
      >
        <div
          className="subtitle-stage"
          style={{
            opacity: subtitleVisible ? 1 : 0,
            transform: subtitleVisible
              ? "translateY(0px) scale(1)"
              : "translateY(8px) scale(0.995)",
            transition: "opacity 320ms ease, transform 320ms ease",
            pointerEvents: "none",
          }}
        >
          <div className="subtitle-text">
            {displayedAnswer || <span className="subtitle-placeholder">.</span>}
          </div>
        </div>

        <div className="bottom-stack">
          <div className="bubble-shell">
            <div className="bubble-row">
              <div className="input-wrap">
                <input
                  ref={inputRef}
                  value={question}
                  onChange={(e) => setQuestion(e.target.value)}
                  className="bubble-input"
                  placeholder="open youtube, open arc raiders, mute, oder frag mich etwas …"
                  onKeyDown={(e) => {
                    if (e.key === "Enter") {
                      e.preventDefault();
                      void handleTypedSubmit();
                    }

                    if (e.key === "Escape") {
                      e.preventDefault();
                      void closeBubble();
                    }
                  }}
                />

                <div className="bubble-meta">
                  <span className="bubble-hint">
                    {busy ? "Verarbeite..." : hint}
                  </span>

                  {interimText ? (
                    <span className="bubble-live">… {interimText}</span>
                  ) : null}
                </div>
              </div>

              <button
                className={`icon-btn sound-btn ${
                  speakEnabled ? "icon-btn-active" : ""
                }`}
                onClick={() => {
                  setSpeakEnabled((prev) => {
                    const next = !prev;
                    if (!next) stopSpeaking();
                    return next;
                  });
                }}
                title={speakEnabled ? "Sprachausgabe an" : "Sprachausgabe aus"}
                aria-label="Sprachausgabe"
                type="button"
              >
                {speakEnabled ? <Volume2 size={18} /> : <VolumeX size={18} />}
              </button>

              <button
                className={`icon-btn ${listening ? "icon-btn-active" : ""}`}
                onClick={() => {
                  if (listening) stopVoiceInput();
                  else void startVoiceInput();
                }}
                title={`Spracherkennung (${voiceShortcut})`}
                disabled={busy}
                aria-label="Spracherkennung"
                type="button"
              >
                {listening ? <MicOff size={18} /> : <Mic size={18} />}
              </button>

              <button
                className="icon-btn"
                onClick={() => void handleTypedSubmit()}
                title="Senden"
                disabled={busy}
                aria-label="Senden"
                type="button"
              >
                <Send size={18} />
              </button>
            </div>
          </div>

          <div className="tiny-links">
            <button
              className="tiny-link"
              onClick={() => void openDevWindow()}
              type="button"
            >
              dev mode
            </button>

            <span
              className="tiny-link tiny-link-static"
              title={`route: ${lastRoute}`}
            >
              {lastRoute === "command"
                ? "befehl ausgeführt"
                : lastRoute === "ollama"
                ? "ollama antwort"
                : "bereit"}
            </span>

            <span className="tiny-link tiny-link-static">
              voice {voiceShortcut}
            </span>

            <span className="tiny-link tiny-link-static">model {model}</span>
          </div>
        </div>
      </div>
    </>
  );
}

createRoot(document.getElementById("root")!).render(<BubbleApp />);
