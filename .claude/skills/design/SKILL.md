---
name: design
description:
  Fetch a GitHub issue, analyze it in the project context, and iterate on the design
---

# Design Issue

Fetch a GitHub issue, analyze it in the project context, and iterate on the design in a
local draft before updating the issue.

## Arguments

- `$ARGUMENTS`: Issue URL or number (e.g., `42`)

## Instructions

### 1. Fetch the issue

```bash
gh issue view <number>
```

### 2. Analyze project context

Read relevant documentation and code:

- `CLAUDE.md` - Project conventions
- Existing trait definitions and patterns
- The project epic (#1) for overall architecture

### 3. Create a local draft

```bash
# File: <issue_number>_draft.md
```

### 4. Design the solution

Structure the design with:

```markdown
## Context

<Why this is needed>

## Proposed Solution

<High-level approach, 3-5 bullet points>

## Trait Signatures

\`\`\`rust
// Show the key traits/structs
\`\`\`

## Implementation Details

### Files to create/modify

- `src/module.rs` - Description

### Type System Constraints

<What the compiler enforces>

## Acceptance Criteria

- [ ] Criterion 1
- [ ] Criterion 2
```

### 5. Break down large features

For complex features, split into multiple PRs:

1. **Create sub-issues** - Each handles one piece
2. **Link issues** - Use "Part of #X"
3. **Create a feature branch** - PRs target feature branch
4. **Final PR** - Feature branch into main

### 6. Verify codebase coherence

- Does this follow existing trait patterns?
- Does the naming match existing conventions?
- Are generics used consistently with the rest of the codebase?

### 7. Review cycle

After each batch of changes:

1. Summarize what was modified
2. Ask if user wants to continue, review, or update the issue

### 8. Update the GitHub issue

Once approved:

```bash
gh issue edit <number> --body "$(cat <issue_number>_draft.md)"
rm <issue_number>_draft.md
```

## Tips

- **Show trait signatures** - They're the contract, make them reviewable
- **Be explicit about type constraints** - What compiles, what doesn't
- **Consider ergonomics** - How does the API feel to use?
- **Keep code snippets minimal** - Show interfaces, not full implementations

$ARGUMENTS