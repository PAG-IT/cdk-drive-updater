# Why

The project has no meaningful README and no FUNCTIONALITY.md, making onboarding and AI-assisted development difficult. Codified rules to keep both documents updated ensure they stay accurate as the codebase evolves.

## What Changes

- Create `README.md` with full human-readable project documentation (overview, requirements, configuration, usage, architecture, components)
- Create `FUNCTIONALITY.md` with a structured, AI-friendly machine-readable description of every module, type, function, and data flow
- Add openspec agent/instruction rules to enforce that both documents are updated whenever source code changes

## Capabilities

### New Capabilities

- `readme-documentation`: Human-facing README covering purpose, prerequisites, environment variables, CLI usage, output format, and architecture overview
- `functionality-documentation`: AI-friendly FUNCTIONALITY.md providing a structured reference of all modules, public types, functions, algorithms, and data flows
- `documentation-maintenance-rules`: Openspec agent rules requiring README.md and FUNCTIONALITY.md to be kept in sync with source code changes

### Modified Capabilities

<!-- None — no existing specs are changing -->

## Impact

- Adds two new documentation files to the repository root
- Adds instruction rules in `openspec/` or `.github/` that apply to all future changes
- No source code changes; no dependency changes; no breaking changes
