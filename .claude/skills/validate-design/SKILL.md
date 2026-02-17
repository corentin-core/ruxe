---
name: validate-design
description:
  Validate the user's design for a GitHub issue — challenge, formalize, and document
---

# Validate Design

The user presents their design for an issue. Claude validates it by challenging
assumptions, identifying gaps, and producing formal documentation.

**Claude does NOT redesign.** Claude challenges the user's design and helps refine it.

## Arguments

- `$ARGUMENTS`: Issue URL or number (e.g., `42`)

## Instructions

### Phase 1: Understand Context

#### Step 1.1: Fetch the issue

```bash
gh issue view <number> --json title,body,labels,state
```

#### Step 1.2: Read relevant code

Use MCP Serena or grep to understand the current codebase state:

- Existing trait definitions and patterns
- Related modules and types
- The project epic (#1) for overall architecture

### Phase 2: Challenge the Design

Ask the user probing questions:

1. **Type safety** — Can the compiler enforce this? Are there runtime checks that could
   be compile-time?
2. **Ownership & lifetimes** — Who owns the data? Are there borrowing issues?
3. **Trait bounds** — Are the constraints tight enough? Too tight?
4. **Alternatives** — "Have you considered X instead of Y?"
5. **Edge cases** — What happens when the collection is empty? When the type doesn't
   implement the trait?

**Do NOT propose a complete alternative design.** Ask questions that guide the user to
discover improvements themselves.

### Phase 3: Formalize

Once the user has addressed the challenges, produce:

#### 3.1: Mermaid diagrams

Use the appropriate diagram type:

- **classDiagram** — trait/struct relationships, associated types
- **sequenceDiagram** — event dispatch flow, middleware chain
- **stateDiagram-v2** — state transitions
- **graph TB** — architecture overview, module dependencies

#### 3.2: Trait signatures summary

```rust
// Show the agreed-upon trait signatures (from the user's design)
```

#### 3.3: Implementation checklist

- [ ] `src/file.rs` — Description
- [ ] `tests/test_file.rs` — Tests

### Phase 4: Update the Issue

Once the user approves the formalized design:

```bash
gh issue edit <number> --body "$(cat <number>_draft.md)"
rm <number>_draft.md
```

## Checkpoints

| After        | Action                                       |
| ------------ | -------------------------------------------- |
| Phase 2      | **WAIT** — User addresses challenges         |
| Phase 3      | **WAIT** — User approves formal documentation|
| Phase 4      | **WAIT** — User confirms issue update        |

## Anti-Patterns

| Anti-Pattern                    | Correct Approach                         |
| ------------------------------- | ---------------------------------------- |
| Proposing a full design         | Ask questions to improve the user's      |
| Skipping challenges             | Always challenge before formalizing       |
| Writing implementation code     | Only show trait signatures, not bodies    |

$ARGUMENTS
