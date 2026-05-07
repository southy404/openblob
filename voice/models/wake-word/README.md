# Wake-word models

This folder is the repo-local development fallback for OpenBlob wake-word model assets.

Runtime/user-installed models should live under:

```text
%APPDATA%/OpenBlob/voice/models/wake-word/
```

Runtime/user-installed ONNX Runtime files should live under:

```text
%APPDATA%/OpenBlob/voice/runtime/onnxruntime/
  onnxruntime.dll
  LICENSE.txt
  VERSION.txt
```

The app checks the runtime AppData model path first, then this repo-local folder while developing. The Dev / Wake Word settings panel can verify the selected runtime and model bundle, open the AppData folders, and save explicit local paths. It does not start the microphone or enable wake-word detection after verification.

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
- `local-openwakeword` / `local-wakeword` discover and validate local model bundles, normalize microphone audio to mono 16 kHz frames, and run compatible local ONNX bundles when the runtime is explicitly configured.

ONNX Runtime is not bundled by OpenBlob. For local development, provide `onnxruntime.dll` in the AppData runtime folder, next to the bundle, under `openwakeword/runtime/onnxruntime.dll`, through Dev UI selection, or through:

```text
OPENBLOB_ONNX_RUNTIME_PATH=C:\path\to\onnxruntime.dll
```

The Dev UI includes explicit download buttons as a safe boundary, but automatic downloads are not configured yet. If/when downloads are added, they must write only to the OpenBlob AppData voice folders, never execute downloaded files, and require user confirmation.

When the runtime and model bundle are present, OpenBlob loads the mel-spectrogram, embedding, and classifier ONNX sessions, feeds transient in-memory audio frames through the local chain, compares the resulting score with the manifest threshold, and emits `wake-word-detected` when the score passes. Model/runtime errors are reported as `runtime_missing`, `invalid_model_bundle`, `unsupported_model_shape`, or `provider_error` rather than being hidden or faked.

Wake-to-voice is optional and controlled separately by `wake_word_auto_listen_enabled`. When enabled, the frontend reacts to a fresh `wake-word-detected` event and starts the same voice input flow used by the manual `ALT + M` shortcut. Detection itself never executes commands directly.

No cloud calls, paid provider keys, automatic model downloads, or raw audio recording files are required for this foundation.

Licensing note: openWakeWord code is open-source, but pretrained model licenses may differ. Verify a model bundle's license before redistribution or commercial use. OpenBlob does not bundle paid or cloud wake-word providers by default.
