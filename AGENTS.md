# AGENTS.md — OpenChamber agents & repository conventions

This file documents the agents available to OpenChamber sessions in this repository and the conventions every session must follow. It is intentionally separate from `sudokooler/AGENTS.md`, which is the Rust daemon submodule's engineering guide.

## Available agents

Agent descriptions are written as Conventional Commit-style subject lines — `<type>(<scope>): <description>` — so they stay short, structured, and consistent with the repository's commit messages.

### `explore`

`feat(agent:explore): fast codebase exploration via file patterns, keyword search, and architecture Q&A`

- Specialized for quick discovery: find files with glob patterns (e.g. `src/components/**/*.tsx`), search code for keywords (e.g. `API endpoints`), and answer questions about the codebase (e.g. "how do API endpoints work?").
- Always specify a thoroughness level when dispatching: `quick` for basic searches, `medium` for moderate exploration, or `very thorough` for comprehensive analysis across multiple locations and naming conventions.
- Use this agent when you need answers about the code without changing it.

### `general`

`feat(agent:general): general-purpose agent for multi-step, cross-cutting tasks`

- Handles work that does not fit a narrow specialty: investigations, refactors spanning many files, and tasks that combine exploration with edits.
- Runs with the session's bound model and starts with a fresh context, so give it a self-contained prompt containing all relevant context and constraints.
- Use this agent to delegate part of your own task. For separate work the user follows and talks to, dispatch a standalone OpenChamber session instead.

## Using agents

- Delegate part of your own task to a subagent; give it everything it needs up front because child sessions do not inherit this conversation.
- Dispatch a standalone session when the work is independent, needs its own worktree, or the user should follow its progress.
- Keep each agent focused: one clear objective, an explicit scope, and a definition of done.

## Commit convention

All commits in this repository MUST follow [Conventional Commits](https://www.conventionalcommits.org/en/v1.0.0/):

```
<type>(<optional scope>): <description>

[optional body]

[optional BREAKING CHANGE footer]
```

- **Types:** `feat`, `fix`, `docs`, `style`, `refactor`, `perf`, `test`, `build`, `ci`, `chore`, `revert`.
- The `<description>` is a short, imperative, lower-case summary with no trailing period.
- Use a scope when it adds clarity, e.g. `feat(sensors): add PawnIO temperature backend`.
- Breaking changes use `!` after the type/scope and/or a `BREAKING CHANGE:` footer.
- Agent descriptions above follow the same subject-line format; keep them in sync when an agent's role changes.
