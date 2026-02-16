---
name: create-pr
description: Create a well-documented GitHub pull request with proper linking to issues
---

# Create Pull Request

Create a well-documented GitHub pull request with proper linking to issues.

## Arguments

- `$ARGUMENTS`: Optional issue number to link (e.g., `#42` or `42`)

## Instructions

### 1. Gather context

```bash
git branch --show-current
git log origin/main..HEAD --oneline
git diff origin/main...HEAD --stat
```

If an issue number is provided, fetch issue details:

```bash
gh issue view <number>
```

### 2. Review the changes

```bash
git diff origin/main...HEAD
```

Identify:

- Main changes and their purpose
- Files that might need explanation
- Trade-offs or design decisions

### 3. Ensure branch is pushed

```bash
git push -u origin $(git branch --show-current)
```

### 4. Create the PR

```bash
gh pr create --title "<type>: <description>" --body "$(cat <<'EOF'
## Summary

<Brief description of what this PR does>

## Related Issue

Closes #<issue_number>

## Changes

- <List of main changes>

## Notes for Reviewers

<Any context that helps reviewers understand the changes>
EOF
)"
```

### 5. Title format

Use conventional commits style:

- `feat: add SliceReducer trait`
- `fix: correct parallel dispatch ordering`
- `refactor: simplify middleware chain`
- `test: add integration tests for RootReducer`
- `docs: add examples to README`

## Tips

- Keep the summary concise but informative
- Link the issue properly (`Closes #X` for auto-close on merge)
- Mention any areas where you'd like specific feedback

$ARGUMENTS