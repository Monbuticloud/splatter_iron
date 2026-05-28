# ADR 1: Record Architecture Decisions

- **Status:** Accepted
- **Date:** 2026-05-28

## Context

We need a consistent way to document deliberate architectural decisions made
for SplatterIron. Without a record, the rationale behind design choices is
lost to tribal knowledge.

## Decision

We will use Architecture Decision Records (ADRs), as described by Michael
Nygard. Each ADR is a short markdown file capturing a single decision.

- ADRs live in `docs/architecture/`
- Numbered sequentially: `0001-title.md`, `0002-title.md`, ...
- Each ADR has: **Title**, **Status**, **Date**, **Context**, **Decision**,
  **Consequences**
- Statuses: `Proposed`, `Accepted`, `Deprecated`, `Superseded`
- A decision is *Accepted* once implemented.

## Consequences

- ADRs are easy to find and review in one place.
- New contributors can understand why the system is built this way.
- ADRs may become stale if not maintained alongside code changes.
