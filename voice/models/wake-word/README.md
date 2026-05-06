# Wake-word models

This folder is the repo-local development fallback for OpenBlob wake-word model assets.

Runtime/user-installed models should live under:

```text
%APPDATA%/OpenBlob/voice/models/wake-word/
```

The app checks the runtime AppData path first, then this repo-local folder while developing.

Supported placeholder extensions for discovery are:

- `.onnx`
- `.tflite`
- `.bin`
- `.json`

The preferred real local provider path is an openWakeWord-style bundle:

```text
voice/models/wake-word/
  openwakeword/
    manifest.json
    melspectrogram.onnx
    embedding.onnx
    hey-openblob.onnx
```

Example manifest:

```json
{
  "id": "hey-openblob",
  "provider": "local-openwakeword",
  "phrase": "hey openblob",
  "runtime": "onnx",
  "sampleRate": 16000,
  "frameMs": 80,
  "threshold": 0.5,
  "models": {
    "melspectrogram": "melspectrogram.onnx",
    "embedding": "embedding.onnx",
    "classifier": "hey-openblob.onnx"
  }
}
```

Large model binaries must not be committed. Add local/user-provided wake-word model files only on your machine.

Current provider behavior:

- `mic-test` only validates the local microphone pipeline and never runs wake-word detection.
- `mock` is dev-only and can simulate detection from loud local input.
- `local-openwakeword` / `local-wakeword` discover and validate local model bundles, normalize microphone audio to mono 16 kHz frames, and report `runtime_missing` until an ONNX inference backend is linked.

Wake-to-voice is optional and controlled separately by `wake_word_auto_listen_enabled`. When enabled, the frontend can react to a `wake-word-detected` event and start the same voice input flow used by the manual `ALT + M` shortcut.

No cloud calls, paid provider keys, automatic model downloads, or raw audio recording files are required for this foundation.
