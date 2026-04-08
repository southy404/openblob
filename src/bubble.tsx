import { useEffect, useMemo, useRef, useState } from "react";
import { createRoot } from "react-dom/client";
import { invoke } from "@tauri-apps/api/core";
import { emit, listen } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";
import {
  Languages,
  MessageCircleQuestion,
  WandSparkles,
  X,
  Volume2,
  VolumeX,
  Mic,
  MicOff,
  Send,
  TerminalSquare,
} from "lucide-react";

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
    0: {
      transcript: string;
    };
  }>;
};

declare global {
  interface Window {
    webkitSpeechRecognition?: new () => SpeechRecognitionLike;
    SpeechRecognition?: new () => SpeechRecognitionLike;
  }
}

function speak(text: string) {
  if (!("speechSynthesis" in window)) return;

  window.speechSynthesis.cancel();
  const utter = new SpeechSynthesisUtterance(text);
  utter.rate = 1;
  utter.pitch = 1;
  utter.lang = "de-DE";
  window.speechSynthesis.speak(utter);
}

function BubbleApp() {
  const [copiedText, setCopiedText] = useState("");
  const [question, setQuestion] = useState("");
  const [answer, setAnswer] = useState("");
  const [hint, setHint] = useState("Bubble bereit.");
  const [model, setModel] = useState("llama3.1:8b");
  const [busy, setBusy] = useState(false);
  const [ollamaReady, setOllamaReady] = useState<boolean | null>(null);
  const [speakEnabled, setSpeakEnabled] = useState(true);

  const [voiceSupported, setVoiceSupported] = useState(false);
  const [listening, setListening] = useState(false);
  const [interimText, setInterimText] = useState("");
  const [autoSendVoice, setAutoSendVoice] = useState(true);
  const [recording, setRecording] = useState(false);

  const recognitionRef = useRef<SpeechRecognitionLike | null>(null);
  const micStreamRef = useRef<MediaStream | null>(null);

  const SpeechRecognitionCtor = useMemo(
    () => window.SpeechRecognition || window.webkitSpeechRecognition || null,
    []
  );

  const stopMicTracks = () => {
    if (micStreamRef.current) {
      micStreamRef.current.getTracks().forEach((track) => track.stop());
      micStreamRef.current = null;
    }
  };

  const runMode = async (
    mode: "translate_de" | "translate_en" | "explain" | "ask",
    textOverride?: string,
    questionOverride?: string
  ) => {
    const sourceText = (textOverride ?? copiedText).trim();
    const sourceQuestion = (questionOverride ?? question).trim();

    if (!sourceText) {
      setHint("Kein Kontexttext vorhanden.");
      return;
    }

    if (mode === "ask" && !sourceQuestion) {
      setHint("Bitte stelle zuerst eine Frage.");
      return;
    }

    setBusy(true);
    setAnswer("");

    try {
      const result = await invoke<OllamaResult>("ask_ollama", {
        mode,
        text: sourceText,
        question: mode === "ask" ? sourceQuestion : null,
        model,
      });

      setAnswer(result.content);
      setHint(`Antwort von ${result.model}`);

      const shortSpeech = result.content.slice(0, 140);
      await emit("companion-speech", shortSpeech);

      if (speakEnabled) {
        speak(result.content.slice(0, 220));
      }
    } catch (error) {
      setHint(String(error));
    } finally {
      setBusy(false);
    }
  };

  const executeCommandOrAsk = async (rawInput: string) => {
    const input = rawInput.trim();

    if (!input) {
      setHint("Bitte gib etwas ein.");
      return;
    }

    setQuestion(input);

    try {
      const actionResult = await invoke<string>("handle_voice_command", {
        input,
      });

      if (actionResult !== "NO_ACTION") {
        setAnswer(actionResult);
        setHint("Systemaktion ausgeführt.");

        await emit("companion-speech", actionResult);

        if (speakEnabled) {
          speak(actionResult);
        }
        return;
      }
    } catch (error) {
      const message = String(error);

      if (message.includes("Prozent-Lautstärke")) {
        setAnswer(message);
        setHint("Befehl erkannt, aber noch nicht exakt implementiert.");
        return;
      }

      setHint(`Command-Fehler: ${message}`);
      return;
    }

    setHint("Kein Systembefehl erkannt. Nutze Ollama...");
    await runMode("ask", input, input);
  };

  const handleTranscript = async (transcriptRaw: string) => {
    const transcript = transcriptRaw.trim();

    if (!transcript) {
      setHint("Keine Sprache erkannt.");
      return;
    }

    await executeCommandOrAsk(transcript);
  };

  const handleTypedSubmit = async () => {
    if (busy) return;
    await executeCommandOrAsk(question);
  };

  const runLocalVoiceInput = async () => {
    if (recording || busy) return;

    try {
      setRecording(true);
      setHint("Nehme lokal auf...");

      const transcript = await invoke<string>("record_and_transcribe_voice", {
        seconds: 5,
      });

      if (!transcript.trim()) {
        setHint("Keine Sprache erkannt.");
        return;
      }

      setHint("Lokale Transkription fertig.");
      await handleTranscript(transcript);
    } catch (error) {
      setHint(`Voice-Fehler: ${String(error)}`);
    } finally {
      setRecording(false);
    }
  };

  const startVoiceInput = async () => {
    if (!SpeechRecognitionCtor) {
      setHint("SpeechRecognition wird hier nicht unterstützt.");
      return;
    }

    if (listening) return;

    try {
      const stream = await navigator.mediaDevices.getUserMedia({
        audio: {
          echoCancellation: true,
          noiseSuppression: true,
        },
        video: false,
      });

      micStreamRef.current = stream;

      const recognition = new SpeechRecognitionCtor();
      recognition.lang = "de-DE";
      recognition.interimResults = true;
      recognition.continuous = false;
      recognition.maxAlternatives = 1;

      recognition.onstart = () => {
        setListening(true);
        setInterimText("");
        setHint("Ich höre zu...");
      };

      recognition.onresult = async (event) => {
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
          const cleaned = finalTranscript.trim();
          setInterimText("");
          setHint("Sprache erkannt.");

          if (autoSendVoice) {
            await handleTranscript(cleaned);
          } else {
            setQuestion(cleaned);
          }
        }
      };

      recognition.onerror = (event) => {
        setListening(false);
        stopMicTracks();
        setHint(`Voice-Fehler: ${event.error ?? "unbekannt"}`);
      };

      recognition.onend = () => {
        setListening(false);
        stopMicTracks();
        recognitionRef.current = null;
        setInterimText("");
      };

      recognitionRef.current = recognition;
      recognition.start();
    } catch (error) {
      stopMicTracks();
      setListening(false);
      setHint(`Mikrofon nicht verfügbar: ${String(error)}`);
    }
  };

  const stopVoiceInput = () => {
    recognitionRef.current?.stop();
    recognitionRef.current = null;
    stopMicTracks();
    setListening(false);
    setInterimText("");
    setHint("Voice gestoppt.");
  };

  useEffect(() => {
    setVoiceSupported(Boolean(SpeechRecognitionCtor));
  }, [SpeechRecognitionCtor]);

  useEffect(() => {
    let unlistenFn: null | (() => void) = null;

    const setup = async () => {
      unlistenFn = await listen<ContextPayload>(
        "companion-context",
        async (event) => {
          const payload = event.payload;
          const text = payload.text || "";

          setCopiedText(text);
          if (payload.hint) setHint(payload.hint);

          if (payload.autoRun && text.trim()) {
            await runMode("explain", text);
          }
        }
      );
    };

    setup();

    return () => {
      if (unlistenFn) unlistenFn();
    };
  }, [model, speakEnabled]);

  useEffect(() => {
    const checkOllama = async () => {
      try {
        const ready = await invoke<boolean>("ping_ollama");
        setOllamaReady(ready);
      } catch {
        setOllamaReady(false);
      }
    };

    checkOllama();
  }, []);

  useEffect(() => {
    return () => {
      try {
        recognitionRef.current?.stop();
      } catch {}
      stopMicTracks();
    };
  }, []);

  const closeBubble = async () => {
    stopVoiceInput();
    await getCurrentWindow().hide();
  };

  return (
    <div className="bubble-shell">
      <div className="bubble-card">
        <div className="bubble-header" data-tauri-drag-region>
          <div className="bubble-badge">
            companion bubble ·{" "}
            {ollamaReady === null
              ? "prüfe..."
              : ollamaReady
              ? "ollama online"
              : "ollama offline"}
          </div>

          <div style={{ display: "flex", gap: 8 }}>
            <button
              className="bubble-chip"
              onClick={runLocalVoiceInput}
              disabled={busy || recording}
            >
              {recording ? <MicOff size={14} /> : <Mic size={14} />}
              {recording ? " Aufnahme..." : " Voice"}
            </button>

            <button
              className="bubble-icon-button"
              onClick={() => setSpeakEnabled((v) => !v)}
              title="Audio an/aus"
            >
              {speakEnabled ? <Volume2 size={16} /> : <VolumeX size={16} />}
            </button>

            <button
              className="bubble-icon-button"
              onClick={closeBubble}
              title="Schließen"
            >
              <X size={16} />
            </button>
          </div>
        </div>

        <div className="bubble-section">
          <div className="bubble-label">Model</div>
          <input
            className="bubble-input"
            value={model}
            onChange={(e) => setModel(e.target.value)}
            placeholder="z. B. llama3.1:8b"
          />
        </div>

        <div className="bubble-section">
          <div className="bubble-label">Kontext</div>
          <div className="bubble-scroll bubble-context">
            {copiedText || "Noch kein Text."}
          </div>
        </div>

        <div className="bubble-actions">
          <button
            className="bubble-chip"
            onClick={() => runMode("translate_de")}
            disabled={busy}
          >
            <Languages size={14} /> DE
          </button>

          <button
            className="bubble-chip"
            onClick={() => runMode("translate_en")}
            disabled={busy}
          >
            <Languages size={14} /> EN
          </button>

          <button
            className="bubble-chip"
            onClick={() => runMode("explain")}
            disabled={busy}
          >
            <WandSparkles size={14} /> Erklären
          </button>
        </div>

        <div className="bubble-section">
          <div className="bubble-label">Voice Input</div>

          <div className="bubble-actions" style={{ marginBottom: 8 }}>
            <button
              className="bubble-chip"
              onClick={listening ? stopVoiceInput : startVoiceInput}
              disabled={!voiceSupported || busy}
              title={
                voiceSupported
                  ? listening
                    ? "Voice stoppen"
                    : "Voice starten"
                  : "SpeechRecognition hier nicht verfügbar"
              }
            >
              {listening ? <MicOff size={14} /> : <Mic size={14} />}
              {listening ? " Stop" : " Voice"}
            </button>

            <button
              className="bubble-chip"
              onClick={() => setAutoSendVoice((v) => !v)}
              disabled={!voiceSupported}
              title="Automatisch nach Erkennung senden"
            >
              <Send size={14} />
              {autoSendVoice ? " Auto-Send an" : " Auto-Send aus"}
            </button>
          </div>

          <div className="bubble-scroll" style={{ maxHeight: 64 }}>
            {!voiceSupported
              ? "SpeechRecognition wird hier nicht unterstützt."
              : listening
              ? interimText || "Ich höre zu..."
              : "Voice bereit."}
          </div>
        </div>

        <div className="bubble-section">
          <div className="bubble-label">Befehl / Frage</div>
          <textarea
            className="bubble-textarea"
            value={question}
            onChange={(e) => setQuestion(e.target.value)}
            placeholder='z. B. spiele "smooth criminal" oder Was bedeutet das konkret?'
            onKeyDown={(e) => {
              if (e.key === "Enter" && !e.shiftKey) {
                e.preventDefault();
                void handleTypedSubmit();
              }
            }}
          />
          <div
            style={{ marginTop: 8, display: "flex", gap: 8, flexWrap: "wrap" }}
          >
            <button
              className="bubble-chip"
              onClick={handleTypedSubmit}
              disabled={busy}
            >
              <TerminalSquare size={14} />
              {busy ? " Arbeite..." : " Ausführen"}
            </button>

            <button
              className="bubble-chip"
              onClick={() => runMode("ask")}
              disabled={busy}
            >
              <MessageCircleQuestion size={14} />
              {busy ? " Denke..." : " Nur Ollama"}
            </button>
          </div>
        </div>

        <div className="bubble-section bubble-answer-section">
          <div className="bubble-label">Antwort</div>
          <div className="bubble-scroll bubble-answer">
            {answer || "Noch keine Antwort."}
          </div>
        </div>

        <div className="bubble-hint">{hint}</div>
      </div>
    </div>
  );
}

createRoot(document.getElementById("root")!).render(<BubbleApp />);
