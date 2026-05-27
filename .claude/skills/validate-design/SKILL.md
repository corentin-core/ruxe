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

#### Separate design feedback from implementation feedback

A design draft is **not** finished code. The user is exploring shape, not polishing
syntax. Mixing the two levels dilutes the review and frustrates the user.

When reviewing a design, classify each remark into one of two buckets:

- **Design-level** (semantic, structural, conceptual) — wrong trait usage, wrong
  abstraction boundary, missing concept, pedagogical mismatch. **These are the
  review.**
- **Implementation-level** (compile errors, missing derives, typos, naming
  placeholders, non-exhaustive matches, position of doc comments) — will be caught
  by the compiler or `cargo check` at implementation time.

Implementation-level remarks should be either:

1. **Omitted entirely** during the design phase — the compiler will surface them, no
   value added by listing them now
2. **Or grouped at the end** under a clearly-labeled section like *"Nits à corriger à
   l'implem (pas bloquant pour le design)"* — so the user can ignore them during the
   design iteration

Never interleave a design question with a "you forgot `#[derive(Clone)]`" remark in the
same priority bucket. The design question carries the cost of an iteration; the missing
derive carries the cost of a compiler hint.

### Phase 3: Formalize

Once the user has addressed the challenges, produce:

- **Mermaid diagrams** (classDiagram, sequenceDiagram, etc.)
- **Trait signatures summary**
- **Implementation checklist**

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

| Anti-Pattern                                | Correct Approach                                  |
| ------------------------------------------- | ------------------------------------------------- |
| Proposing a full design                     | Ask questions to improve the user's               |
| Skipping challenges                         | Always challenge before formalizing               |
| Writing implementation code                 | Only show trait signatures, not bodies            |
| Mixing design + compile-error feedback      | Separate buckets; omit or footnote impl-level     |

$ARGUMENTS
