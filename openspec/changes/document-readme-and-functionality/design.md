# Context

The repository currently has a three-line README.md stub and no FUNCTIONALITY.md. There are no rules that require either file to be updated when source code changes. Developers and AI agents working on the codebase have no structured reference for how the application works.

The application is a Windows-only Rust CLI tool that:

1. Reads the CDK Drive OSD (Online Software Distribution) catalog by scraping an HTML page
2. Reads the installed state from the Windows registry and filesystem
3. Compares installed vs. available versions for four target software packages
4. Logs a rich ASCII-table report to stdout and a timestamped log file

## Goals / Non-Goals

**Goals:**

- Write a human-readable README.md that serves as the first stop for any developer or operator
- Write a machine-readable FUNCTIONALITY.md that gives AI agents a complete, structured reference to the codebase without requiring them to read every source file
- Add agent instruction rules in AGENTS.md or a `.instructions.md` file that enforce both documents are kept in sync with source changes
- The spec covering `documentation-maintenance-rules` describes what the rules must cover

**Non-Goals:**

- Changes to source code
- Automated CI enforcement (optional future work)
- Internationalization or multi-language documentation

## Decisions

### D1 — README targets human readers; FUNCTIONALITY.md targets AI agents

The two files serve different audiences and are kept separate so each can be optimized for its reader. README uses prose, badges, and examples; FUNCTIONALITY.md uses structured headings in a consistent format that is easy to parse programmatically.

**Alternative considered:** A single large document — rejected because blending human and machine audiences produces a document that serves neither well.

### D2 — Maintenance rules live in AGENTS.md

The project already has `openspec/AGENTS.md`. The rules for keeping documentation updated will be added there (for the AI agent context) and mirrored in a `.github/copilot-instructions.md` or a `.copilot/instructions/` file so VS Code Copilot picks them up automatically.

**Alternative considered:** A standalone `.instructions.md` in the root — acceptable, but AGENTS.md is already the designated AI-instruction entry point for this project and should stay the single source of truth.

### D3 — FUNCTIONALITY.md uses a module-per-section structure

Each Rust source file (`main.rs`, `installed.rs`, `cdk_info.rs`, `app_logging.rs`) gets its own section with: purpose, public types, public functions/signatures, and key internal logic. A top-level "Data Flow" section describes the end-to-end execution path.

## Risks / Trade-offs

- [Risk] FUNCTIONALITY.md becomes stale if contributors don't follow the rules → Mitigation: the maintenance rules in AGENTS.md are explicit and tied to file-change triggers
- [Risk] README becomes too long and hard to navigate → Mitigation: use clear H2 sections and a table of contents
