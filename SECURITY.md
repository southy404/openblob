# Security Policy

If you discover a security issue, please do not open a public issue.

Instead:

- open a private report (GitHub security tab)
- or contact maintainers directly

---

## Scope

Relevant areas include:

- browser automation
- system commands
- local execution
- external integrations

---

## Notes

OpenBlob runs locally but still interacts with system and browser — security matters.

---

## ⚠️ Security & Antivirus Notice

OpenBlob is a **local-first desktop application with deep system integration**.

Because of its capabilities, some antivirus or Windows security systems may flag or block parts of the application.

This is expected behavior due to:

- global keyboard shortcuts
- screen capture & snipping
- input simulation (keyboard / mouse)
- active window & process inspection
- browser automation (remote debugging)
- local AI execution
- system audio capture for transcript sessions

---

### What this means

- Windows Defender or other antivirus tools **may warn or block execution**
- SmartScreen may show **"unknown publisher" warnings**
- Some features (like browser control or input simulation) may be restricted

---

### What you can do

If you trust the project:

- allow the app through Windows Defender
- add an exclusion/whitelist for the OpenBlob directory
- ensure Chrome/Edge debugging port (9222) is not blocked
- run the app with sufficient permissions if needed

---

### Transparency

OpenBlob is:

- **open-source** — you can inspect everything
- **local-first** — no hidden cloud processing
- **explicit about system access**

No data is sent externally unless explicitly triggered (e.g. APIs or model calls you configure).

---

> ⚠️ Always review the code before running software that interacts deeply with your system.
