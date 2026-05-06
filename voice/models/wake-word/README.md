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

Large model binaries must not be committed. Add local/user-provided wake-word model files only on your machine. The current local provider discovers models and reports setup status; real open-source inference is intentionally left for a future provider implementation.
