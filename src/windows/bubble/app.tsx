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
type UiLang = "en" | "de";
type RouteState = "command" | "ollama" | "none";

type BlobPhase =
  | "idle"
  | "listening"
  | "transcript"
  | "thinking"
  | "executing"
  | "alert";

type BlobSignal = Exclude<BlobPhase, "idle">;

type BubbleTexts = {
  ready: string;
  processing: string;
  pleaseEnterSomething: string;
  justChatting: string;
  knownSiteOpened: string;
  knownSiteOpeningSpoken: (url: string) => string;
  hideAndSeekStarted: string;
  hideAndSeekStartedSpoken: string;
  commandExecuted: string;
  directCommandRecognizedButNoLocalMatch: string;
  noLocalCommandMatched: string;
  localCommandFailed: string;
  listening: string;
  speechRecognitionUnsupported: string;
  voiceError: (msg: string) => string;
  microphoneError: (msg: string) => string;
  chattingWithModel: (model: string) => string;
  answerFromModel: (model: string) => string;
  errorPrefix: (msg: string) => string;
  directCommandFailedMessage: (input: string) => string;
  placeholderChat: string;
  placeholderCommand: string;
  ttsOnTitle: string;
  ttsOffTitle: string;
  speechRecognitionTitle: (shortcut: string) => string;
  sendTitle: string;
  devMode: string;
  routeReady: string;
  routeCommand: string;
  routeOllama: string;
  subtitlesLabel: string;
  modeLabel: string;
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
  subtitlesEnabled: "openblob-bubble-subtitles-enabled",
  bubbleMode: "openblob-bubble-mode",
};

const BLOB_SIGNALS: BlobSignal[] = [
  "listening",
  "transcript",
  "thinking",
  "executing",
  "alert",
];

const BUBBLE_TEXTS: Record<UiLang, BubbleTexts> = {
  en: {
    ready: "Ready.",
    processing: "Processing...",
    pleaseEnterSomething: "Please enter something.",
    justChatting: "Just chatting...",
    knownSiteOpened: "Opened known site directly.",
    knownSiteOpeningSpoken: (url) => `Opening ${url}.`,
    hideAndSeekStarted: "Hide and seek started.",
    hideAndSeekStartedSpoken: "Okay, hide and seek started. Find me.",
    commandExecuted: "Command executed.",
    directCommandRecognizedButNoLocalMatch:
      "Command recognized, but nothing matching was found locally.",
    noLocalCommandMatched: "No local command matched. Asking Ollama...",
    localCommandFailed: "Local command failed.",
    listening: "Listening…",
    speechRecognitionUnsupported: "Speech recognition is not supported here.",
    voiceError: (msg) => `Voice error: ${msg}`,
    microphoneError: (msg) => `Microphone error: ${msg}`,
    chattingWithModel: (model) => `Chatting with ${model}`,
    answerFromModel: (model) => `Answer from ${model}`,
    errorPrefix: (msg) => `Error: ${msg}`,
    directCommandFailedMessage: (input) =>
      `Could not execute the local command: "${input}"`,
    placeholderChat: "talk to me…",
    placeholderCommand: "open youtube, mute, or ask me something…",
    ttsOnTitle: "Speech output on",
    ttsOffTitle: "Speech output off",
    speechRecognitionTitle: (shortcut) => `Speech recognition (${shortcut})`,
    sendTitle: "Send",
    devMode: "dev mode",
    routeReady: "ready",
    routeCommand: "command executed",
    routeOllama: "ollama response",
    subtitlesLabel: "subtitles",
    modeLabel: "mode",
  },
  de: {
    ready: "Bereit.",
    processing: "Verarbeite...",
    pleaseEnterSomething: "Bitte gib etwas ein.",
    justChatting: "Einfach chatten...",
    knownSiteOpened: "Bekannte Seite direkt geöffnet.",
    knownSiteOpeningSpoken: (url) => `Öffne ${url}.`,
    hideAndSeekStarted: "Hide and seek gestartet.",
    hideAndSeekStartedSpoken: "Okay, hide and seek gestartet. Finde mich.",
    commandExecuted: "Befehl ausgeführt.",
    directCommandRecognizedButNoLocalMatch:
      "Befehl erkannt, aber lokal nichts Passendes gefunden.",
    noLocalCommandMatched: "Kein lokaler Befehl erkannt. Frage Ollama...",
    localCommandFailed: "Lokaler Befehl fehlgeschlagen.",
    listening: "Ich höre zu …",
    speechRecognitionUnsupported:
      "Spracherkennung wird hier nicht unterstützt.",
    voiceError: (msg) => `Sprachfehler: ${msg}`,
    microphoneError: (msg) => `Mikrofonfehler: ${msg}`,
    chattingWithModel: (model) => `Chatte mit ${model}`,
    answerFromModel: (model) => `Antwort von ${model}`,
    errorPrefix: (msg) => `Fehler: ${msg}`,
    directCommandFailedMessage: (input) =>
      `Konnte den lokalen Befehl nicht ausführen: "${input}"`,
    placeholderChat: "rede mit mir …",
    placeholderCommand: "open youtube, mute oder frag mich etwas …",
    ttsOnTitle: "Sprachausgabe an",
    ttsOffTitle: "Sprachausgabe aus",
    speechRecognitionTitle: (shortcut) => `Spracherkennung (${shortcut})`,
    sendTitle: "Senden",
    devMode: "dev mode",
    routeReady: "bereit",
    routeCommand: "befehl ausgeführt",
    routeOllama: "ollama antwort",
    subtitlesLabel: "subtitles",
    modeLabel: "mode",
  },
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
  const [uiLang, setUiLang] = useState<UiLang>("en");
  const questionRef = useRef("");
  const [hint, setHint] = useState(BUBBLE_TEXTS.en.ready);
  const [model, setModel] = useState(
    readLocalStorageString(STORAGE_KEYS.model, "llama3.1:8b")
  );
  const [busy, setBusy] = useState(false);
  const [ollamaElapsedMs, setOllamaElapsedMs] = useState<number | null>(null);
  const [lastShortcut, setLastShortcut] = useState<string | null>(null);
  const [isMacOS] = useState(() =>
    /Mac|iPhone|iPad|iPod/i.test(navigator.userAgent)
  );
  const [visible, setVisible] = useState(false);
  const [listening, setListening] = useState(false);
  const [interimText, setInterimText] = useState("");
  const [lastRoute, setLastRoute] = useState<RouteState>("none");
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
  const busyRef = useRef(false);
  const phaseRef = useRef<BlobPhase>("idle");

  const t = BUBBLE_TEXTS[uiLang];

  const SpeechRecognitionCtor = useMemo(
    () => window.SpeechRecognition || window.webkitSpeechRecognition || null,
    []
  );

  const emitBlobState = async (state: BlobSignal, active: boolean) => {
    try {
      await emit("blob-state", { state, active });
    } catch {}
  };

  const sleep = (ms: number) =>
    new Promise((resolve) => window.setTimeout(resolve, ms));

  const setBlobPhase = async (phase: BlobPhase) => {
    phaseRef.current = phase;

    for (const state of BLOB_SIGNALS) {
      await emitBlobState(state, phase !== "idle" && state === phase);
    }
  };

  const showThinkingBeforeExecuting = async () => {
    setHint(t.processing);
    await setBlobPhase("thinking");
    await sleep(260);
  };

  const flashBlobAlert = async () => {
    await setBlobPhase("alert");

    window.setTimeout(() => {
      if (phaseRef.current === "alert") {
        void setBlobPhase("idle");
      }
    }, 1600);
  };

  const loadIdentity = async () => {
    try {
      const result = (await invoke("get_identity")) as [string, string, string];
      const [, , lang] = result;
      setUiLang(lang === "de" ? "de" : "en");
    } catch (error) {
      console.error("failed to load identity for bubble ui", error);
      setUiLang("en");
    }
  };

  useEffect(() => {
    if (isMacOS) {
      void invoke("clear_glass_effect", { window: getCurrentWindow() }).catch(
        () => {}
      );
      return;
    }

    const applyGlass = async () => {
      try {
        const win = getCurrentWindow();
        await invoke("apply_glass_effect", { window: win });
      } catch (error) {
        console.error("failed to apply glass effect", error);
      }
    };

    void applyGlass();
  }, [isMacOS]);

  useEffect(() => {
    void loadIdentity();
  }, []);

  useEffect(() => {
    let unlisten: null | (() => void) = null;

    const setup = async () => {
      unlisten = await listen("identity-updated", async () => {
        await loadIdentity();
      });
    };

    void setup();

    return () => {
      unlisten?.();
    };
  }, []);

  useEffect(() => {
    setHint((current) => {
      if (
        current === BUBBLE_TEXTS.en.ready ||
        current === BUBBLE_TEXTS.de.ready
      ) {
        return t.ready;
      }

      return current;
    });
  }, [uiLang, t.ready]);

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
    const onBlur = () => {
      if (!visibleRef.current) return;
      inputRef.current?.blur();
      void fadeOutAndHide();
    };

    window.addEventListener("blur", onBlur);
    return () => window.removeEventListener("blur", onBlur);
  }, []);

  useEffect(() => {
    listeningRef.current = listening;
  }, [listening]);

  useEffect(() => {
    busyRef.current = busy;
  }, [busy]);

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
    const focused = await win.isFocused().catch(() => false);
    if (focused) focusInputSoon();
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
    await setBlobPhase("thinking");

    const started = Date.now();
    setOllamaElapsedMs(0);
    const tick = window.setInterval(() => {
      setOllamaElapsedMs(Date.now() - started);
    }, 250);

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
          ? t.chattingWithModel(result.model)
          : t.answerFromModel(result.model)
      );
      setLastRoute("ollama");

      await emit("companion-speech", result.content.slice(0, 180));

      if (speakEnabled) {
        await speak(result.content.slice(0, 260), setHint);
      }
    } finally {
      window.clearInterval(tick);
      setOllamaElapsedMs(null);
      await setBlobPhase("idle");
    }
  };

  const clearComposer = () => {
    questionRef.current = "";
    setInterimText("");

    window.setTimeout(() => {
      if (inputRef.current) {
        inputRef.current.value = "";
      }
    }, 0);
  };

  const executeCommandOrAsk = async (rawInput: string) => {
    const input = rawInput.trim();

    if (!input) {
      setHint(t.pleaseEnterSomething);
      return;
    }

    if (busyRef.current) return;

    busyRef.current = true;
    setBusy(true);
    await setBlobPhase("thinking");

    try {
      if (bubbleMode === "chat") {
        setHint(t.justChatting);
        await runOllamaAsk(input);
        return;
      }

      const directUrl = getDirectKnownUrl(input);

      if (directUrl) {
        await showThinkingBeforeExecuting();
        await setBlobPhase("executing");

        await invoke<string>("handle_voice_command", {
          input: `open ${directUrl}`,
        });

        const spoken = t.knownSiteOpeningSpoken(directUrl);
        await showSubtitle(spoken, 4200);
        setHint(t.knownSiteOpened);
        setLastRoute("command");

        if (speakEnabled) {
          void speak(spoken);
        }

        await setBlobPhase("idle");
        return;
      }

      if (isHideAndSeekCommand(input)) {
        await showThinkingBeforeExecuting();
        await setBlobPhase("executing");

        await emit("start-hide-and-seek");
        await showSubtitle(t.hideAndSeekStartedSpoken, 4200);
        setHint(t.hideAndSeekStarted);
        setLastRoute("command");

        if (speakEnabled) {
          void speak(t.hideAndSeekStartedSpoken);
        }

        await setBlobPhase("idle");
        return;
      }

      await setBlobPhase("thinking");

      const actionResult = await invoke<string>("handle_voice_command", {
        input,
      });

      if (actionResult !== "NO_ACTION") {
        await setBlobPhase("executing");
        await sleep(450);

        await showSubtitle(actionResult, 4200);
        setHint(t.commandExecuted);
        setLastRoute("command");

        await emit("companion-speech", actionResult);

        if (speakEnabled) {
          await speak(actionResult.slice(0, 220));
        }

        return;
      }

      await setBlobPhase("thinking");

      if (looksLikeDirectCommand(input)) {
        const message = t.directCommandFailedMessage(input);

        await showSubtitle(message, 4200);
        setHint(t.directCommandRecognizedButNoLocalMatch);
        setLastRoute("command");

        if (speakEnabled) {
          await speak(message);
        }

        await flashBlobAlert();
        return;
      }

      setHint(t.noLocalCommandMatched);
      await runOllamaAsk(input);
    } catch (error) {
      const message = String(error);

      await showSubtitle(message, 4800);

      if (bubbleMode === "command" && looksLikeDirectCommand(input)) {
        setHint(t.localCommandFailed);
        setLastRoute("command");
      } else {
        setHint(t.errorPrefix(message));
      }

      if (speakEnabled) {
        await speak(message.slice(0, 220));
      }

      await flashBlobAlert();
    } finally {
      busyRef.current = false;
      setBusy(false);

      if (phaseRef.current !== "alert") {
        await setBlobPhase("idle");
      }
    }
  };

  const handleTypedSubmit = async () => {
    if (busyRef.current) return;

    const text = (questionRef.current || "").trim();

    if (!text) {
      setHint(t.pleaseEnterSomething);
      return;
    }

    clearComposer();
    await executeCommandOrAsk(text);
  };

  const startVoiceInput = async () => {
    if (!SpeechRecognitionCtor) {
      setHint(t.speechRecognitionUnsupported);
      await flashBlobAlert();
      return;
    }

    if (listeningRef.current || busyRef.current) return;

    try {
      const recognition = new SpeechRecognitionCtor();
      recognition.lang = uiLang === "de" ? "de-DE" : "en-US";
      recognition.interimResults = true;
      recognition.continuous = false;
      recognition.maxAlternatives = 1;

      recognition.onstart = async () => {
        finalVoiceTextRef.current = "";
        setListening(true);
        setInterimText("");
        setHint(t.listening);
        await setBlobPhase("listening");
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

        if (liveTranscript.trim() || finalTranscript.trim()) {
          void setBlobPhase("transcript");
        }

        if (finalTranscript.trim()) {
          const text = finalTranscript.trim();
          finalVoiceTextRef.current = text;
          questionRef.current = text;
          if (inputRef.current) inputRef.current.value = text;
        }
      };

      recognition.onend = async () => {
        setListening(false);
        listeningRef.current = false;
        recognitionRef.current = null;

        const finalText = finalVoiceTextRef.current.trim();
        finalVoiceTextRef.current = "";

        clearComposer();

        if (finalText) {
          await setBlobPhase("thinking");
          await executeCommandOrAsk(finalText);
        } else {
          await setBlobPhase("idle");
        }
      };

      recognition.onerror = async (event) => {
        setListening(false);
        listeningRef.current = false;
        recognitionRef.current = null;
        setInterimText("");
        finalVoiceTextRef.current = "";
        setHint(t.voiceError(event.error ?? "unknown"));
        await flashBlobAlert();
      };

      recognitionRef.current = recognition;
      recognition.start();
    } catch (error) {
      setListening(false);
      listeningRef.current = false;
      setInterimText("");
      finalVoiceTextRef.current = "";
      setHint(t.microphoneError(String(error)));
      await flashBlobAlert();
    }
  };

  const stopVoiceInput = () => {
    try {
      recognitionRef.current?.stop();
    } catch {}

    recognitionRef.current = null;
    setListening(false);
    listeningRef.current = false;
    setInterimText("");

    void setBlobPhase("idle");
  };

  useEffect(() => {
    let unlistenContext: null | (() => void) = null;
    let unlistenToggle: null | (() => void) = null;
    let unlistenShow: null | (() => void) = null;
    let unlistenHide: null | (() => void) = null;
    let unlistenVoiceToggle: null | (() => void) = null;
    let unlistenShortcut: null | (() => void) = null;

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
            const next = payload.text.trim();
            questionRef.current = next;
            if (inputRef.current) inputRef.current.value = next;
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

      unlistenShortcut = await listen<string>("debug-shortcut", (event) => {
        const value = String(event.payload || "");
        if (!value) return;
        setLastShortcut(value);
        window.setTimeout(() => setLastShortcut(null), 1800);
      });
    };

    void setup();

    return () => {
      unlistenContext?.();
      unlistenToggle?.();
      unlistenShow?.();
      unlistenHide?.();
      unlistenVoiceToggle?.();
      unlistenShortcut?.();
    };
  }, [uiLang, t]);

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
  }, [voiceShortcut, uiLang, t]);

  useEffect(() => {
    return () => {
      void stopSpeaking();
      void emitTo("bubble-subtitle", "bubble-subtitle-clear").catch(() => {});
      void setBlobPhase("idle");

      try {
        recognitionRef.current?.stop();
      } catch {}
    };
  }, []);

  const toggleBubbleMode = () => {
    setBubbleMode((prev) => {
      const next: BubbleMode = prev === "command" ? "chat" : "command";
      setHint(next === "chat" ? t.justChatting : t.ready);
      return next;
    });
  };

  const placeholder =
    bubbleMode === "chat" ? t.placeholderChat : t.placeholderCommand;

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

        .macos-lite .bubble-shell {
          backdrop-filter: none;
          -webkit-backdrop-filter: none;
          background: rgba(24, 24, 28, 0.28);
          box-shadow:
            inset 0 1px 1px rgba(255, 255, 255, 0.18),
            inset 0 -1px 1px rgba(0, 0, 0, 0.16);
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

        .macos-lite .send-glow-line,
        .macos-lite .send-glow-blur {
          filter: none;
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
          from { stroke-dashoffset: 0; }
          to { stroke-dashoffset: -100; }
        }

        @keyframes sendGlowVisibility {
          0%, 100% { opacity: 0.08; }
          18% { opacity: 0.95; }
          50% { opacity: 1; }
          82% { opacity: 0.92; }
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
        className={`bubble-stage${isMacOS ? " macos-lite" : ""}`}
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
                  defaultValue=""
                  onChange={(e) => {
                    questionRef.current = e.target.value;
                  }}
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
                  <span>{busy ? t.processing : hint}</span>
                  {ollamaElapsedMs !== null && (
                    <span> | Ollama {Math.ceil(ollamaElapsedMs / 1000)}s</span>
                  )}
                  {lastShortcut && <span> | key {lastShortcut}</span>}
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
                title={speakEnabled ? t.ttsOnTitle : t.ttsOffTitle}
                type="button"
              >
                {speakEnabled ? <Volume2 size={20} /> : <VolumeX size={20} />}
              </button>

              <button
                className={`icon-btn ${listening ? "icon-btn-active" : ""}`}
                onClick={() => {
                  if (listeningRef.current) stopVoiceInput();
                  else void startVoiceInput();
                }}
                title={t.speechRecognitionTitle(voiceShortcut)}
                disabled={busy}
                type="button"
              >
                {listening ? <MicOff size={20} /> : <Mic size={20} />}
              </button>

              <button
                className="icon-btn send-btn"
                onClick={() => void handleTypedSubmit()}
                title={t.sendTitle}
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
              {t.devMode}
            </button>

            <span className="tiny-link tiny-link-static">
              {lastRoute === "command"
                ? t.routeCommand
                : lastRoute === "ollama"
                ? t.routeOllama
                : t.routeReady}
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
              {t.subtitlesLabel} {subtitlesEnabled ? "on" : "off"}
            </button>

            <button
              className="tiny-link"
              onClick={toggleBubbleMode}
              type="button"
            >
              {t.modeLabel} {bubbleMode}
            </button>
          </div>
        </div>
      </div>
    </>
  );
}

createRoot(document.getElementById("root")!).render(<BubbleApp />);
