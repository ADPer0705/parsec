# Security Notes

## 🛡️ Principles

- **Least Surprise:** Always display exact commands for execution.
- **Explicit Consent:** Zero auto-execution of AI suggestions.
- **Isolation (Future):** Sandboxed/namespaced execution for high-risk commands.

## 🎯 Threat Model

- Prompt injection attacks producing destructive commands.
- Model hallucinations generating dangerous operations (e.g., `rm /`).
- Secret leakage through command echoes and logging.

## 🚨 Near-Term Mitigations

- Mandatory approval for all generated commands.
- Warning banners for destructive patterns (e.g., `rm -rf /`).

## 🔮 Future Work

- Static risk scoring for commands.
- Dry-run simulation support (git, cargo, etc.) where available.
