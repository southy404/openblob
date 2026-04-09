import { useEffect, useMemo, useRef, useState } from "react";
import { createRoot } from "react-dom/client";
import { invoke } from "@tauri-apps/api/core";
import { emit, listen } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";
import {
  MessageCircleQuestion,
  X,
  Volume2,
  VolumeX,
  Mic,
  MicOff,
  Send,
  Sparkles,
  Bot,
  Command,
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
  const [hint, setHint] = useState("Bereit.");
  const [model, setModel] = useState("llama3.1:8b");
  const [busy, setBusy] = useState(false);
  const [ollamaReady, setOllamaReady] = useState<boolean | null>(null);
  const [speakEnabled, setSpeakEnabled] = useState(true);

  const [voiceSupported, setVoiceSupported] = useState(false);
  const [listening, setListening] = useState(false);
  const [interimText, setInterimText] = useState("");
  const [autoSendVoice, setAutoSendVoice] = useState(true);
  const [recording, setRecording] = useState(false);

  const [visible, setVisible] = useState(false);
  const [mounted, setMounted] = useState(true);
  const [showAnswer, setShowAnswer] = useState(false);
  const [showContext, setShowContext] = useState(false);

  const recognitionRef = useRef<SpeechRecognitionLike | null>(null);
  const micStreamRef = useRef<MediaStream | null>(null);
  const inputRef = useRef<HTMLTextAreaElement | null>(null);

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

  const focusInputSoon = () => {
    window.setTimeout(() => {
      inputRef.current?.focus();
      inputRef.current?.setSelectionRange(
        inputRef.current.value.length,
        inputRef.current.value.length
      );
    }, 120);
  };

  const fadeOutAndHide = async () => {
    setVisible(false);
    window.setTimeout(async () => {
      await getCurrentWindow().hide();
    }, 180);
  };

  const fadeInAndShow = async () => {
    await getCurrentWindow().show();
    setMounted(true);
    requestAnimationFrame(() => {
      setVisible(true);
    });
    await getCurrentWindow().setFocus();
    focusInputSoon();
  };

  const runMode = async (
    mode: "translate_de" | "translate_en" | "explain" | "ask",
    textOverride?: string,
    questionOverride?: string
  ) => {
    const sourceText = (textOverride ?? copiedText).trim();
    const sourceQuestion = (questionOverride ?? question).trim();

    if (!sourceText && mode !== "ask") {
      setHint("Kein Kontext vorhanden.");
      return;
    }

    if (mode === "ask" && !sourceQuestion) {
      setHint("Bitte zuerst etwas eingeben.");
      return;
    }

    setBusy(true);
    setAnswer("");
    setShowAnswer(true);

    try {
      const result = await invoke<OllamaResult>("ask_ollama", {
        mode,
        text: sourceText || sourceQuestion,
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
    setShowAnswer(true);

    if (isHideAndSeekCommand(input)) {
      await emit("start-hide-and-seek");
      setAnswer("Okay, hide and seek started. Find me.");
      setHint("Hide and seek started.");
      return;
    }

    try {
      const actionResult = await invoke<string>("handle_voice_command", {
        input,
      });

      if (actionResult !== "NO_ACTION") {
        setAnswer(actionResult);
        setHint("Befehl ausgeführt.");

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

    setHint("Kein Systembefehl erkannt. Frage die KI...");
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
            focusInputSoon();
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
    let unlistenContext: null | (() => void) = null;
    let unlistenHotkey: null | (() => void) = null;

    const setup = async () => {
      unlistenContext = await listen<ContextPayload>(
        "companion-context",
        async (event) => {
          const payload = event.payload;
          const text = payload.text || "";

          await fadeInAndShow();

          setCopiedText(text);
          if (payload.hint) setHint(payload.hint);

          if (payload.autoRun && text.trim()) {
            setShowAnswer(true);
            await runMode("explain", text);
          }
        }
      );
    };

    setup();

    return () => {
      if (unlistenContext) unlistenContext();
      if (unlistenHotkey) unlistenHotkey();
    };
  }, [visible, model, speakEnabled]);

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
    await fadeOutAndHide();
  };

  return (
    <div
      className="bubble-window-shell"
      style={{
        opacity: visible ? 1 : 0,
        transform: visible ? "translateY(0px)" : "translateY(24px)",
        transition: "opacity 220ms ease, transform 220ms ease",
        pointerEvents: visible ? "auto" : "none",
      }}
    >
      <div className="bubble-window-card glass-panel bubble-no-drag">
        <div
          style={{
            position: "relative",
            zIndex: 1,
            display: "grid",
            gridTemplateColumns: "1fr auto",
            gap: 12,
            alignItems: "center",
          }}
        >
          <div
            data-tauri-drag-region
            className="bubble-drag-region"
            style={{
              display: "flex",
              alignItems: "center",
              gap: 12,
              minHeight: 42,
              padding: "0 4px",
            }}
          >
            <div
              style={{
                width: 40,
                height: 40,
                borderRadius: 14,
                background:
                  "linear-gradient(180deg, rgba(126,167,255,0.24), rgba(126,167,255,0.08))",
                border: "1px solid rgba(146,184,255,0.24)",
                display: "grid",
                placeItems: "center",
                boxShadow: "0 10px 30px rgba(78,117,217,0.24)",
                flexShrink: 0,
              }}
            >
              <Sparkles size={18} />
            </div>

            <div style={{ minWidth: 0 }}>
              <div
                style={{
                  fontSize: 15,
                  fontWeight: 800,
                  letterSpacing: 0.2,
                }}
              >
                Ask anything
              </div>
              <div
                style={{
                  fontSize: 12,
                  opacity: 0.62,
                  whiteSpace: "nowrap",
                  overflow: "hidden",
                  textOverflow: "ellipsis",
                }}
              >
                {ollamaReady === null
                  ? "prüfe ollama..."
                  : ollamaReady
                  ? `online · ${model}`
                  : "ollama offline"}
              </div>
            </div>
          </div>

          <div
            className="bubble-no-drag"
            style={{ display: "flex", gap: 8, alignItems: "center" }}
          >
            <button
              onClick={() => setShowContext((v) => !v)}
              style={topGhostButtonStyle}
              title="Kontext ein-/ausblenden"
            >
              <Command size={16} />
            </button>

            <button
              onClick={() => setSpeakEnabled((v) => !v)}
              style={topGhostButtonStyle}
              title="Audio an/aus"
            >
              {speakEnabled ? <Volume2 size={16} /> : <VolumeX size={16} />}
            </button>

            <button
              onClick={closeBubble}
              style={topGhostButtonStyle}
              title="Ausblenden"
            >
              <X size={16} />
            </button>
          </div>
        </div>

        {showContext && (
          <div
            style={{
              position: "relative",
              zIndex: 1,
              marginTop: 12,
            }}
          >
            <div className="glass-section">
              <div style={labelStyle}>Kontext</div>
              <div className="glass-scroll" style={contextBoxStyle}>
                {copiedText || "Noch kein Kontext."}
              </div>
            </div>
          </div>
        )}

        <div
          style={{
            position: "relative",
            zIndex: 1,
            marginTop: 12,
          }}
        >
          <div className="glass-section" style={{ padding: 14 }}>
            <textarea
              ref={inputRef}
              className="glass-input"
              value={question}
              onChange={(e) => setQuestion(e.target.value)}
              placeholder='"Search [X] on YouTube", "Play Rick and Morty on Netflix", "Start Arc Raiders"'
              onKeyDown={(e) => {
                if (e.key === "Enter" && !e.shiftKey) {
                  e.preventDefault();
                  void handleTypedSubmit();
                }
              }}
              style={{
                minHeight: 112,
                maxHeight: 112,
                lineHeight: 1.5,
                border: "none",
                boxShadow: "none",
                padding: 0,
                background: "transparent",
                fontSize: 18,
                resize: "none",
              }}
            />

            <div
              style={{
                marginTop: 14,
                display: "flex",
                alignItems: "center",
                justifyContent: "space-between",
                gap: 12,
                flexWrap: "wrap",
              }}
            >
              <div
                style={{
                  display: "flex",
                  gap: 8,
                  alignItems: "center",
                  flexWrap: "wrap",
                }}
              >
                <button
                  onClick={runLocalVoiceInput}
                  disabled={busy || recording}
                  style={bottomIconButtonStyle}
                  title="Lokale Sprachaufnahme"
                >
                  {recording ? <MicOff size={17} /> : <Mic size={17} />}
                </button>

                <button
                  onClick={listening ? stopVoiceInput : startVoiceInput}
                  disabled={!voiceSupported || busy}
                  style={bottomGhostChipStyle}
                >
                  {listening ? <MicOff size={14} /> : <Mic size={14} />}
                  {listening ? "Stop" : "Voice"}
                </button>

                <button
                  onClick={() => setAutoSendVoice((v) => !v)}
                  disabled={!voiceSupported}
                  style={bottomGhostChipStyle}
                >
                  <Send size={14} />
                  {autoSendVoice ? "Auto-Send an" : "Auto-Send aus"}
                </button>

                <button
                  onClick={() => runMode("ask")}
                  disabled={busy}
                  style={bottomGhostChipStyle}
                >
                  <Bot size={14} />
                  KI fragen
                </button>
              </div>

              <div
                style={{
                  display: "flex",
                  gap: 8,
                  alignItems: "center",
                  marginLeft: "auto",
                }}
              >
                <button
                  onClick={handleTypedSubmit}
                  disabled={busy}
                  style={primaryPillStyle}
                >
                  <MessageCircleQuestion size={15} />
                  {busy ? "Arbeite..." : "Ausführen"}
                </button>
              </div>
            </div>
          </div>
        </div>

        {(showAnswer || answer) && (
          <div
            style={{
              position: "relative",
              zIndex: 1,
              marginTop: 12,
            }}
          >
            <div className="glass-section">
              <div
                style={{
                  display: "flex",
                  justifyContent: "space-between",
                  alignItems: "center",
                  marginBottom: 8,
                }}
              >
                <div style={labelStyle}>Antwort</div>
                <button
                  onClick={() => setShowAnswer((v) => !v)}
                  style={miniFlatButtonStyle}
                >
                  {showAnswer ? "Einklappen" : "Ausklappen"}
                </button>
              </div>

              {showAnswer && (
                <div className="glass-scroll" style={answerBoxStyle}>
                  {answer || "Noch keine Antwort."}
                </div>
              )}
            </div>
          </div>
        )}

        <div
          style={{
            position: "relative",
            zIndex: 1,
            marginTop: 10,
            display: "flex",
            justifyContent: "space-between",
            alignItems: "center",
            gap: 12,
            flexWrap: "wrap",
          }}
        >
          <div
            style={{
              fontSize: 12,
              opacity: 0.72,
              minHeight: 18,
            }}
          >
            {listening
              ? interimText || "Ich höre zu..."
              : recording
              ? "Lokale Aufnahme läuft..."
              : hint}
          </div>

          <div
            style={{
              fontSize: 11,
              opacity: 0.48,
            }}
          >
            Shortcut: Ctrl + Space
          </div>
        </div>
      </div>
    </div>
  );
}

const labelStyle: React.CSSProperties = {
  fontSize: 12,
  fontWeight: 700,
  opacity: 0.82,
  marginBottom: 8,
  letterSpacing: 0.2,
};

const contextBoxStyle: React.CSSProperties = {
  maxHeight: 88,
  overflow: "auto",
  borderRadius: 16,
  padding: "12px 14px",
  background: "rgba(255,255,255,0.03)",
  border: "1px solid rgba(255,255,255,0.06)",
  fontSize: 13,
  lineHeight: 1.45,
  color: "rgba(238,244,255,0.88)",
  whiteSpace: "pre-wrap",
};

const answerBoxStyle: React.CSSProperties = {
  minHeight: 138,
  maxHeight: 220,
  overflow: "auto",
  borderRadius: 16,
  padding: "12px 14px",
  background: "rgba(255,255,255,0.03)",
  border: "1px solid rgba(255,255,255,0.06)",
  fontSize: 13,
  lineHeight: 1.55,
  whiteSpace: "pre-wrap",
  color: "rgba(238,244,255,0.92)",
};

const topGhostButtonStyle: React.CSSProperties = {
  width: 38,
  height: 38,
  display: "grid",
  placeItems: "center",
  borderRadius: 14,
  border: "1px solid rgba(255,255,255,0.08)",
  background: "rgba(255,255,255,0.05)",
  color: "#eef4ff",
  cursor: "pointer",
};

const bottomIconButtonStyle: React.CSSProperties = {
  width: 42,
  height: 42,
  display: "grid",
  placeItems: "center",
  borderRadius: 999,
  border: "1px solid rgba(255,255,255,0.08)",
  background:
    "linear-gradient(180deg, rgba(255,255,255,0.08), rgba(255,255,255,0.04))",
  color: "#eef4ff",
  cursor: "pointer",
  flexShrink: 0,
};

const bottomGhostChipStyle: React.CSSProperties = {
  display: "inline-flex",
  alignItems: "center",
  gap: 8,
  height: 40,
  padding: "0 14px",
  borderRadius: 999,
  border: "1px solid rgba(255,255,255,0.08)",
  background: "rgba(255,255,255,0.05)",
  color: "#eef4ff",
  cursor: "pointer",
  fontSize: 13,
  fontWeight: 700,
};

const primaryPillStyle: React.CSSProperties = {
  display: "inline-flex",
  alignItems: "center",
  gap: 8,
  height: 42,
  padding: "0 16px",
  borderRadius: 999,
  border: "1px solid rgba(140,180,255,0.24)",
  background:
    "linear-gradient(180deg, rgba(117,163,255,0.24), rgba(117,163,255,0.12))",
  color: "#eef4ff",
  cursor: "pointer",
  fontSize: 13,
  fontWeight: 800,
  boxShadow: "0 10px 30px rgba(62,106,214,0.2)",
};

const miniFlatButtonStyle: React.CSSProperties = {
  display: "inline-flex",
  alignItems: "center",
  justifyContent: "center",
  height: 30,
  padding: "0 10px",
  borderRadius: 999,
  border: "1px solid rgba(255,255,255,0.08)",
  background: "rgba(255,255,255,0.04)",
  color: "#eef4ff",
  cursor: "pointer",
  fontSize: 12,
  fontWeight: 700,
};

createRoot(document.getElementById("root")!).render(<BubbleApp />);
