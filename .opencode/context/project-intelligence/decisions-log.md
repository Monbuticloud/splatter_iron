<!-- Context: project-intelligence/decisions | Priority: high | Version: 1.0 | Updated: 2026-06-05 -->

# Decisions Log

> Record major architectural and business decisions with full context. This prevents "why was this done?" debates.

## Quick Reference

- **Purpose**: Document decisions so future team members understand context
- **Format**: Each decision as a separate entry
- **Status**: Decided | Pending | Under Review | Deprecated

## Decision Template

```markdown
## [Decision Title]

**Date**: YYYY-MM-DD
**Status**: [Decided/Pending/Under Review/Deprecated]
**Owner**: [Who owns this decision]

### Context
[What situation prompted this decision? What was the problem or opportunity?]

### Decision
[What was decided? Be specific about the choice made.]

### Rationale
[Why this decision? What were the alternatives and why were they rejected?]

### Alternatives Considered
| Alternative | Pros | Cons | Why Rejected? |
|-------------|------|------|---------------|
| [Alt 1] | [Pros] | [Cons] | [Why not chosen] |
| [Alt 2] | [Pros] | [Cons] | [Why not chosen] |

### Impact
**Positive**: [What this enables or improves]
**Negative**: [What trade-offs or limitations this creates]
**Risk**: [What could go wrong]

### Related
- [Links to related decisions, PRs, issues, or documentation]
```

---

## Decision: [Title]

**Date**: YYYY-MM-DD
**Status**: [Status]
**Owner**: [Owner]

### Context
[What was happening? Why did we need to decide?]

### Decision
[What we decided]

### Rationale
[Why this was the right choice]

### Alternatives Considered
| Alternative | Pros | Cons | Why Rejected? |
|-------------|------|------|---------------|
| [Option A] | [Good things] | [Bad things] | [Reason] |
| [Option B] | [Good things] | [Bad things] | [Reason] |

### Impact
- **Positive**: [What we gain]
- **Negative**: [What we trade off]
- **Risk**: [What to watch for]

### Related
- [Link to PR #000]
- [Link to issue #000]
- [Link to documentation]

---

## Decision: [Title]

**Date**: YYYY-MM-DD
**Status**: [Status]
**Owner**: [Owner]

### Context
[What was happening?]

### Decision
[What we decided]

### Rationale
[Why this was right]

### Alternatives Considered
| Alternative | Pros | Cons | Why Rejected? |
|-------------|------|------|---------------|
| [Option A] | [Good things] | [Bad things] | [Reason] |

### Impact
- **Positive**: [What we gain]
- **Negative**: [What we trade off]

### Related
- [Link]

---

## Decision: Relax atomic commit rules to one-coherent-step principle

**Date**: 2026-06-05
**Status**: Decided
**Owner**: Team

### Context
The original atomic commit rules used a rigid category list — one function, one docstring, one test, one struct, one `impl` block each had to be separate commits. In Rust this is impractical:
- A struct + its `impl` can't compile independently
- A new module doesn't work without a `mod` declaration
- A function + its docstring triggers clippy warnings if separated
- Splitting a function from its tests is pointless when tests reference the function's types

The old rule forced non-compiling intermediate commits, making bisecting and reverting unreliable.

### Decision
Replace the rigid category list with a principle-based rule:
- A commit must compile + pass `cargo clippy` (compilation boundary)
- **Naming test**: describe the commit in one short sentence without `and`/`also` — if you can't, split
- Group freely within that boundary (struct+impl+docstring ✓, module+mod declaration ✓, function+tests ✓)
- `docs/src/` updates remain per-function — one commit per function changed
- Config changes and bug fixes remain separate from feature work

### Rationale
Preserves granularity while accepting Rust's compilation dependencies. The naming test provides an objective, memorable check that's easy to apply. Keeps the spirit of atomic commits (no giant batches, no unrelated mixing) without the impractical pedantry.

### Alternatives Considered
| Alternative | Pros | Cons | Why Rejected? |
|-------------|------|------|---------------|
| Keep strict category list | Maximally granular | Non-compiling commits; high friction | Unworkable in Rust |
| Compilation-boundary exception only | Simple | Still overly strict; docstring rule forces clippy failures | Doesn't fix all pain points |
| Naming test approach (chosen) | Objective, memorable, practical | Slightly looser grouping | Best balance |

### Impact
**Positive**: Commits compile and pass clippy; less friction; clearer rule
**Negative**: Slightly looser grouping in edge cases; trusts agent judgment
**Risk**: Agents might over-group; mitigated by naming test + pre-commit audit

### Related
- [AGENTS.md commit: `87d2e4e`](https://github.com/.../commit/87d2e4e)

---

Decisions that were later overturned (for historical context):

| Decision | Date | Replaced By | Why |
|----------|------|-------------|-----|
| [Old decision] | [Date] | [New decision] | [Reason] |

## Onboarding Checklist

- [ ] Understand the philosophy behind major architectural choices
- [ ] Know why certain technologies were chosen over alternatives
- [ ] Understand trade-offs that were made
- [ ] Know where to find decision context when questions arise
- [ ] Understand what decisions are pending and why

## Related Files

- `technical-domain.md` - Technical implementation affected by these decisions
- `business-tech-bridge.md` - How decisions connect business and technical
- `living-notes.md` - Current open questions that may become decisions
