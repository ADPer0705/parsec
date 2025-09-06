# Security Notes (Early Draft)

## Principles
- **Least Surprise:** Always display exactly what will be executed.
- **Explicit Consent:** No automatic execution of AI suggestions.
- **Isolation (Future):** Namespaced/sandboxed execution for high-risk commands.

## Threat Model
- Prompt injection attacks causing destructive command generation.
- Model hallucination producing dangerous operations (rm / destructive commands).
- Secret leakage through command echo and logging.

## Near-Term Mitigations
- Mandatory manual approval step for all generated commands.
- Warning banner if command matches destructive patterns (e.g., `rm -rf /`).

## Future Work
- Static risk scoring system for commands.
- Dry-run simulation support (git, cargo, etc.) where available.
