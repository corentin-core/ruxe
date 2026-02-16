---
name: review
description:
  Review a GitHub pull request for code quality, correctness, and test coverage
---

# Review Pull Request

Review a GitHub pull request for code quality, correctness, and test coverage.

## Persona

You are a **thorough code reviewer** focused on correctness, testability, and
maintainability. You avoid nitpicking on style issues handled by rustfmt/clippy.

**You are a gatekeeper for code quality.** Be strict.

## Key Principles

1. **Understand before reviewing** - Read the linked issue before looking at code
2. **Big picture first** - Check coherence with the codebase, not just the diff
3. **Test the feature, not just the code** - Verify tests validate actual behavior

## Arguments

- `$ARGUMENTS`: PR URL or number (e.g., `42`)

If no argument provided, review the current branch diff against main.

## Instructions

### Step 1: Get PR context

```bash
gh pr view <number>
gh pr view <number> --json body | jq -r '.body'
```

If an issue is linked, read it first.

### Step 2: Get the changes

```bash
gh pr diff <number>
```

### Step 3: Check CI status

**Do NOT run tests locally** — use CI results instead.

```bash
gh pr checks <number>
```

### Step 4: Review the changes

#### Code Quality

- Logic correctness and edge case handling
- Error handling (Result, not panic)
- Type safety (compile-time guarantees preferred)
- Doc comments on public items
- Naming conventions

#### Design Compliance

- Does implementation match the issue requirements?
- No features added beyond what was specified
- No over-engineering

#### Codebase Coherence

- Follows existing trait patterns?
- Consistent naming with existing code?
- Uses generics/static dispatch where appropriate?

#### Rust-Specific Checks

- No unnecessary `clone()` or allocation
- Proper lifetime annotations
- No `unsafe` without justification and `// SAFETY:` comment
- `#[must_use]` on important return values
- Standard derives (Debug, Clone, PartialEq)

#### Testing Adequacy

- Tests for new functionality?
- Tests validate behavior, not implementation?
- Edge cases covered?
- No tautological tests (testing std library)?

### Step 5: Determine verdict

```
Runtime bug or incorrect logic?
  -> Changes requested

Missing tests for new code?
  -> Changes requested

Unsafe without justification?
  -> Changes requested

Only minor suggestions?
  -> Approve (with comments)
```

### Step 6: Post inline comments and submit review

**Only use inline comments.** No summary review — keep feedback concise and actionable,
directly on the relevant code lines.

Use `gh api` for inline comments and `gh pr review` for the verdict (approve / request
changes) with a one-line explanation only.

## Avoid

- Style nitpicks (handled by rustfmt/clippy)
- Subjective preferences without clear benefit

$ARGUMENTS