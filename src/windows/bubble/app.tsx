import { useEffect, useMemo, useRef, useState } from "react";
import { createRoot } from "react-dom/client";
import { invoke } from "@tauri-apps/api/core";
import { listen, emit, emitTo } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { Mic, MicOff, Send, Volume2, VolumeX } from "lucide-react";
import { ensureDevWindow } from "../bubble-dev/open";
import { ensureSubtitleWindow } from "../bubble-subtitle/open";

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

type BubbleMode = "command" | "chat";

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
  subtitlesEnabled: "openblob-bubble-subtitles-enabled",
  bubbleMode: "openblob-bubble-mode",
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

function readBubbleMode(): BubbleMode {
  const value = readLocalStorageString(STORAGE_KEYS.bubbleMode, "command");
  return value === "chat" ? "chat" : "command";
}

async function speak(text: string, onError?: (msg: string) => void) {
  const trimmed = text.trim();
  if (!trimmed) return;

  try {
    await invoke("speak_text", { text: trimmed });
  } catch (error) {
    const msg = `TTS failed: ${String(error)}`;
    console.error("native tts failed", error);
    onError?.(msg);
  }
}

async function stopSpeaking() {
  try {
    await invoke("stop_tts");
  } catch (error) {
    console.error("stop tts failed", error);
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
  const [subtitlesEnabled, setSubtitlesEnabled] = useState(
    readLocalStorageBool(STORAGE_KEYS.subtitlesEnabled, true)
  );
  const [bubbleMode, setBubbleMode] = useState<BubbleMode>(readBubbleMode());

  const inputRef = useRef<HTMLInputElement | null>(null);
  const recognitionRef = useRef<SpeechRecognitionLike | null>(null);
  const finalVoiceTextRef = useRef("");
  const visibleRef = useRef(visible);
  const listeningRef = useRef(listening);

  const SpeechRecognitionCtor = useMemo(
    () => window.SpeechRecognition || window.webkitSpeechRecognition || null,
    []
  );

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
    const prepareSubtitleWindow = async () => {
      try {
        const subtitleWindow = await ensureSubtitleWindow();
        await subtitleWindow.show().catch(() => {});
        await new Promise((resolve) => window.setTimeout(resolve, 80));
        await subtitleWindow.hide().catch(() => {});
      } catch (error) {
        console.error("failed to prepare subtitle window", error);
      }
    };

    void prepareSubtitleWindow();
  }, []);

  useEffect(() => {
    visibleRef.current = visible;
  }, [visible]);

  useEffect(() => {
    listeningRef.current = listening;
  }, [listening]);

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
    try {
      window.localStorage.setItem(
        STORAGE_KEYS.subtitlesEnabled,
        String(subtitlesEnabled)
      );
    } catch {}
  }, [subtitlesEnabled]);

  useEffect(() => {
    try {
      window.localStorage.setItem(STORAGE_KEYS.bubbleMode, bubbleMode);
    } catch {}
  }, [bubbleMode]);

  useEffect(() => {
    if (!subtitlesEnabled) {
      void emitTo("bubble-subtitle", "bubble-subtitle-clear").catch(() => {});
    }
  }, [subtitlesEnabled]);

  const showSubtitle = async (text: string, holdMs = 5200) => {
    if (!subtitlesEnabled) {
      try {
        await emitTo("bubble-subtitle", "bubble-subtitle-clear");
      } catch {}
      return;
    }

    try {
      const subtitleWindow = await ensureSubtitleWindow();
      await subtitleWindow.show().catch(() => {});
      await new Promise((resolve) => window.setTimeout(resolve, 80));

      await emitTo("bubble-subtitle", "bubble-subtitle-show", {
        text,
        holdMs,
      });
    } catch (error) {
      console.error("failed to show subtitle window", error);
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
        mode: bubbleMode === "chat" ? "chat" : "ask",
        text: prompt,
        question: prompt,
        model,
      });

      await showSubtitle(result.content, 5600);

      setHint(
        bubbleMode === "chat"
          ? `Chatting with ${result.model}`
          : `Antwort von ${result.model}`
      );
      setLastRoute("ollama");

      await emit("companion-speech", result.content.slice(0, 180));

      if (speakEnabled) {
        await speak(result.content.slice(0, 260), setHint);
      }
    } finally {
      await emitBlobState("thinking", false);
    }
  };

  const clearComposer = () => {
    setQuestion("");
    setInterimText("");

    window.setTimeout(() => {
      if (inputRef.current) {
        inputRef.current.value = "";
      }
    }, 0);
  };

  const executeCommandOrAsk = async (rawInput: string) => {
    const input = rawInput.trim();

    if (!input || busy) {
      setHint("Bitte gib etwas ein.");
      return;
    }

    setBusy(true);

    try {
      if (bubbleMode === "chat") {
        setHint("Just chatting...");
        await runOllamaAsk(input);
        return;
      }

      const directUrl = getDirectKnownUrl(input);

      if (directUrl) {
        await invoke<string>("handle_voice_command", {
          input: `open ${directUrl}`,
        });

        await showSubtitle(`Öffne ${directUrl}.`, 4200);
        setHint("Bekannte Seite direkt geöffnet.");
        setLastRoute("command");

        if (speakEnabled) {
          void speak(`Öffne ${directUrl}.`);
        }
        return;
      }

      if (isHideAndSeekCommand(input)) {
        await emit("start-hide-and-seek");
        await showSubtitle("Okay, hide and seek started. Find me.", 4200);
        setHint("Hide and seek started.");
        setLastRoute("command");

        if (speakEnabled) {
          void speak("Okay, hide and seek started. Find me.");
        }
        return;
      }

      const actionResult = await invoke<string>("handle_voice_command", {
        input,
      });

      if (actionResult !== "NO_ACTION") {
        await showSubtitle(actionResult, 4200);
        setHint("Befehl ausgeführt.");
        setLastRoute("command");

        await emit("companion-speech", actionResult);

        if (speakEnabled) {
          await speak(actionResult.slice(0, 220));
        }

        return;
      }

      if (looksLikeDirectCommand(input)) {
        const message = `Konnte den lokalen Befehl nicht ausführen: "${input}"`;
        await showSubtitle(message, 4200);
        setHint("Befehl erkannt, aber lokal nichts Passendes gefunden.");
        setLastRoute("command");

        if (speakEnabled) {
          await speak(message);
        }
        return;
      }

      setHint("Kein lokaler Befehl erkannt. Frage Ollama...");
      await runOllamaAsk(input);
    } catch (error) {
      const message = String(error);

      await showSubtitle(message, 4800);

      if (bubbleMode === "command" && looksLikeDirectCommand(input)) {
        setHint("Lokaler Befehl fehlgeschlagen.");
        setLastRoute("command");
      } else {
        setHint(`Fehler: ${message}`);
      }

      if (speakEnabled) {
        await speak(message.slice(0, 220));
      }
    } finally {
      setBusy(false);
    }
  };

  const handleTypedSubmit = async () => {
    if (busy) return;

    const text = question.trim();
    if (!text) {
      setHint("Bitte gib etwas ein.");
      return;
    }

    clearComposer();
    await executeCommandOrAsk(text);
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
        finalVoiceTextRef.current = "";
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
          const text = finalTranscript.trim();
          finalVoiceTextRef.current = text;
          setQuestion(text);
        }
      };

      recognition.onend = async () => {
        setListening(false);
        recognitionRef.current = null;

        await emitBlobState("listening", false);

        const finalText = finalVoiceTextRef.current.trim();
        finalVoiceTextRef.current = "";

        clearComposer();

        if (finalText) {
          await executeCommandOrAsk(finalText);
        }
      };

      recognition.onerror = async (event) => {
        setListening(false);
        recognitionRef.current = null;
        setInterimText("");
        finalVoiceTextRef.current = "";
        setHint(`Voice error: ${event.error ?? "unbekannt"}`);
        await emitBlobState("listening", false);
      };

      recognitionRef.current = recognition;
      recognition.start();
    } catch (error) {
      setListening(false);
      setInterimText("");
      finalVoiceTextRef.current = "";
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
            await showSubtitle(payload.text.trim(), 5200);
          }

          if (payload.hint) {
            setHint(payload.hint);
          } else if (payload.text?.trim()) {
            setHint(payload.text.trim());
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

        if (isVisible) {
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

        if (listeningRef.current) {
          stopVoiceInput();
        } else {
          await startVoiceInput();
        }
      });
    };

    void setup();

    return () => {
      unlistenContext?.();
      unlistenToggle?.();
      unlistenShow?.();
      unlistenHide?.();
      unlistenVoiceToggle?.();
    };
  }, []);

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

        if (listeningRef.current) stopVoiceInput();
        else void startVoiceInput();
      }
    };

    window.addEventListener("keydown", onKeyDown);
    return () => window.removeEventListener("keydown", onKeyDown);
  }, [voiceShortcut]);

  useEffect(() => {
    return () => {
      void stopSpeaking();
      void emitTo("bubble-subtitle", "bubble-subtitle-clear").catch(() => {});

      try {
        recognitionRef.current?.stop();
      } catch {}
    };
  }, []);

  const toggleBubbleMode = () => {
    setBubbleMode((prev) => {
      const next: BubbleMode = prev === "command" ? "chat" : "command";
      setHint(
        next === "chat" ? "Just chatting mode aktiv." : "Command mode aktiv."
      );
      return next;
    });
  };

  const placeholder =
    bubbleMode === "chat"
      ? "rede mit mir …"
      : "open youtube, mute, oder frag mich etwas …";

  return (
    <>
      <style>{`
        :root {
          color-scheme: dark;
          --text-main: #ffffff;
          --text-soft: rgba(255, 255, 255, 0.72);
          --text-dim: rgba(255, 255, 255, 0.5);
        }

        html,
        body,
        #root {
          width: 100%;
          height: 100%;
          margin: 0;
          background: transparent;
          overflow: hidden;
          font-family: -apple-system, BlinkMacSystemFont, "SF Pro Display", "Segoe UI", sans-serif;
          color: var(--text-main);
        }

        * {
          box-sizing: border-box;
        }

        .bubble-stage {
          width: 100%;
          height: 100%;
          display: flex;
          justify-content: flex-end;
          align-items: center;
          padding: 0 18px 18px;
        }

        .bottom-stack {
          width: min(1040px, calc(100vw - 28px));
          display: flex;
          flex-direction: column;
          align-items: center;
          gap: 10px;
        }

        .bubble-shell {
          width: 100%;
          position: relative;
          border-radius: 999px;
          isolation: isolate;
          background: rgba(24, 24, 28, 0.28);
          backdrop-filter: blur(18px) saturate(155%);
          -webkit-backdrop-filter: blur(18px) saturate(155%);
          border: 1px solid rgba(255, 255, 255, 0.14);
          box-shadow:
            inset 0 1px 1px rgba(255, 255, 255, 0.18),
            inset 0 -1px 1px rgba(0, 0, 0, 0.16);
          backface-visibility: hidden;
        }

        .bubble-row {
          display: grid;
          grid-template-columns: 1fr auto auto auto;
          align-items: center;
          gap: 12px;
          min-height: 86px;
          padding: 12px 16px 12px 24px;
        }

        .input-wrap {
          min-width: 0;
          display: flex;
          flex-direction: column;
          justify-content: center;
          gap: 4px;
        }

        .bubble-input {
          width: 100%;
          height: 40px;
          border: 0;
          outline: none;
          background: transparent;
          color: #ffffff;
          font-size: 19px;
          font-weight: 500;
          padding: 0;
          text-rendering: optimizeLegibility;
        }

        .bubble-input::placeholder {
          color: var(--text-dim);
        }

        .bubble-meta {
          display: flex;
          gap: 10px;
          align-items: center;
          font-size: 12px;
          color: rgba(255, 255, 255, 0.62);
          min-height: 16px;
          font-weight: 400;
        }

        .icon-btn {
          width: 52px;
          height: 52px;
          border-radius: 50%;
          border: 1px solid rgba(255, 255, 255, 0.12);
          background: rgba(255, 255, 255, 0.09);
          color: var(--text-main);
          display: grid;
          place-items: center;
          cursor: pointer;
          transition: all 0.2s cubic-bezier(0.2, 0, 0.2, 1);
          position: relative;
          isolation: isolate;
        }

        .icon-btn:hover {
          background: rgba(255, 255, 255, 0.16);
          border-color: rgba(255, 255, 255, 0.24);
          transform: scale(1.04);
        }

        .icon-btn:active {
          transform: scale(0.96);
        }

        .icon-btn:disabled {
          opacity: 0.5;
          cursor: default;
          transform: none;
        }

        .icon-btn-active {
          background: rgba(255, 255, 255, 0.2);
          border-color: rgba(255, 255, 255, 0.28);
        }
          
        .send-btn {
          position: relative;
          border: none;
          border-radius: 50%;
          background: rgba(255, 255, 255, 0.09);
          box-shadow:
            inset 0 1px 1px rgba(255,255,255,0.22),
            inset 0 -1px 1px rgba(0,0,0,0.16),
            0 0 0 1px rgba(255,255,255,0.08);
          overflow: visible;
          isolation: isolate;

          --glow-line-color: rgba(255,255,255,1);
          --glow-accent-color: rgba(255,255,255,0.98);
          --glow-line-thickness: 1px;
          --glow-blur-size: 5px;
          --glow-speed: 1650ms;
        }

        .send-btn:hover {
          background: rgba(255, 255, 255, 0.16);
          transform: scale(1.045);
        }

        .send-glow {
          pointer-events: none;
          position: absolute;
          inset: -2px;
          width: calc(100% + 4px);
          height: calc(100% + 4px);
          opacity: 0;
          z-index: 1;
          overflow: visible;
        }

        .send-glow-line,
        .send-glow-blur {
          fill: none;
          stroke-linecap: round;
          vector-effect: non-scaling-stroke;
          transform-origin: 50% 50%;
        }

        .send-glow-line {
          stroke: var(--glow-line-color);
          stroke-width: var(--glow-line-thickness);
          stroke-dasharray: 20 30 20 30;
          stroke-dashoffset: 0;
          opacity: 0.92;
          filter:
            drop-shadow(0 0 1px rgba(255,255,255,0.92))
            drop-shadow(0 0 3px rgba(255,255,255,0.38));
        }

        .send-glow-blur {
          stroke: var(--glow-accent-color);
          stroke-width: var(--glow-blur-size);
          stroke-dasharray: 20 30 20 30;
          stroke-dashoffset: 0;
          opacity: 0.72;
          filter: blur(6px);
        }

        .send-btn:hover .send-glow,
        .send-btn:focus-visible .send-glow {
          animation: sendGlowVisibility var(--glow-speed) ease-in-out infinite;
        }

        .send-btn:hover .send-glow-line,
        .send-btn:hover .send-glow-blur,
        .send-btn:focus-visible .send-glow-line,
        .send-btn:focus-visible .send-glow-blur {
          animation: sendGlowOrbit 2400ms cubic-bezier(0.45, 0.05, 0.55, 0.95) infinite;
        }

        .send-btn:disabled .send-glow {
          opacity: 0;
        }

        @keyframes sendGlowOrbit {
          from {
            stroke-dashoffset: 0;
          }
          to {
            stroke-dashoffset: -100;
          }
        }

        @keyframes sendGlowVisibility {
          0%, 100% {
            opacity: 0.08;
          }
          18% {
            opacity: 0.95;
          }
          50% {
            opacity: 1;
          }
          82% {
            opacity: 0.92;
          }
        }
        .tiny-links {
          width: 100%;
          display: flex;
          justify-content: center;
          gap: 16px;
          flex-wrap: wrap;
          margin-top: 4px;
        }

        .tiny-link {
          appearance: none;
          border: 0;
          background: transparent;
          padding: 0;
          font-size: 12px;
          color: rgba(255, 255, 255, 0.5);
          cursor: pointer;
          transition: color 0.2s;
        }

        .tiny-link:hover {
          color: rgba(255, 255, 255, 0.9);
        }

        .tiny-link-static {
          cursor: default;
        }

        @media (max-width: 820px) {
          .bottom-stack {
            width: calc(100vw - 16px);
          }

          .bubble-row {
            grid-template-columns: 1fr auto auto;
            padding: 10px 14px 10px 18px;
            min-height: 76px;
          }

          .icon-btn {
            width: 46px;
            height: 46px;
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
          transition:
            "opacity 180ms ease, transform 180ms cubic-bezier(0.175, 0.885, 0.32, 1.1)",
          pointerEvents: visible ? "auto" : "none",
        }}
      >
        <div className="bottom-stack">
          <div className="bubble-shell">
            <div className="bubble-row">
              <div className="input-wrap">
                <input
                  ref={inputRef}
                  value={question}
                  onChange={(e) => setQuestion(e.target.value)}
                  className="bubble-input"
                  placeholder={placeholder}
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
                  <span>{busy ? "Verarbeite..." : hint}</span>
                  {interimText && <span>| … {interimText}</span>}
                </div>
              </div>

              <button
                className={`icon-btn sound-btn ${
                  speakEnabled ? "icon-btn-active" : ""
                }`}
                onClick={() => {
                  setSpeakEnabled((prev) => {
                    const next = !prev;
                    if (!next) void stopSpeaking();
                    return next;
                  });
                }}
                title={speakEnabled ? "Sprachausgabe an" : "Sprachausgabe aus"}
                type="button"
              >
                {speakEnabled ? <Volume2 size={20} /> : <VolumeX size={20} />}
              </button>

              <button
                className={`icon-btn ${listening ? "icon-btn-active" : ""}`}
                onClick={() => {
                  if (listening) stopVoiceInput();
                  else void startVoiceInput();
                }}
                title={`Spracherkennung (${voiceShortcut})`}
                disabled={busy}
                type="button"
              >
                {listening ? <MicOff size={20} /> : <Mic size={20} />}
              </button>

              <button
                className="icon-btn send-btn"
                onClick={() => void handleTypedSubmit()}
                title="Senden"
                disabled={busy}
                type="button"
              >
                <Send
                  size={18}
                  color="#ffffff"
                  style={{
                    marginLeft: "-1px",
                    position: "relative",
                    zIndex: 2,
                  }}
                />

                <svg
                  className="send-glow"
                  viewBox="0 0 52 52"
                  aria-hidden="true"
                >
                  <circle
                    className="send-glow-blur"
                    cx="26"
                    cy="26"
                    r="24.5"
                    pathLength="100"
                  />
                  <circle
                    className="send-glow-line"
                    cx="26"
                    cy="26"
                    r="24.5"
                    pathLength="100"
                  />
                </svg>
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

            <span className="tiny-link tiny-link-static">
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

            <button
              className="tiny-link"
              onClick={() => {
                setSubtitlesEnabled((prev) => !prev);
              }}
              type="button"
            >
              subtitles {subtitlesEnabled ? "on" : "off"}
            </button>

            <button
              className="tiny-link"
              onClick={toggleBubbleMode}
              type="button"
            >
              mode {bubbleMode}
            </button>
          </div>
        </div>
      </div>
    </>
  );
}

createRoot(document.getElementById("root")!).render(<BubbleApp />);
