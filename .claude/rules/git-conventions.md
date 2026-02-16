# Git Conventions

## Pre-commit Validation

**BEFORE committing changes**, you MUST:

1. **Run tests** to verify your changes don't break existing functionality
2. **Run linters** to ensure code quality

```bash
# Run tests first
cargo test

# Run clippy
cargo clippy -- -D warnings

# Then commit
git add <files> && git commit -m "message"
```

Never commit without running tests first, even for "simple" changes.

## Branch Creation

**ALWAYS use git worktrees** to isolate work from the main working directory.

```bash
# GOOD - create a worktree from main
git worktree add ../ruxe-42 -b issue/42-new-feature origin/main

# BAD - checkout in the main worktree (blocks other work)
git checkout -b issue/42-new-feature
```

Worktree path convention: `../ruxe-<issue-number>`.

Cleanup after merge:

```bash
git worktree remove ../ruxe-42
```

## Branch Naming

Format: `issue/<number>-<kebab-case-description>` or `feature/<number>-<description>`

Example: `issue/8-parallel-root-reducer`

## Commit Messages

Conventional Commits format:

```
type: Description using sentence case without terminal dot
```

**Types:**

| Type     | Purpose                                               |
| -------- | ----------------------------------------------------- |
| feat     | New feature                                           |
| fix      | Bug fix                                               |
| refactor | Code changes that neither fix a bug nor add a feature |
| test     | Adding or updating tests                              |
| docs     | Documentation updates                                 |
| chore    | Maintenance tasks                                     |

**Examples:**

```bash
# Good
feat: add SliceReducer trait with associated Slice type
fix: correct state snapshot ordering in parallel dispatch
refactor: simplify middleware chain composition

# Bad
fix: bug fix                    # Too vague
update code                     # No type specified
Added new feature.              # Wrong casing, unnecessary period
```

## Pull Request Merging

**NEVER merge a PR without explicit user approval.**

After creating a PR:

1. Share the PR URL with the user
2. Wait for the user to review the code and CI checks
3. Only merge when the user explicitly says to merge

## Important

- **NEVER commit directly to main** - always create a feature branch
- **NEVER use `git add -A` or `git add .`** - stage files explicitly
- **NEVER merge PRs without explicit user approval**
- **NEVER add `Co-Authored-By: Claude` to commit messages**
- Use imperative mood: "add feature" not "added feature"
- Keep first line under 50 characters
- Reference issue numbers in the body if relevant

---

For PR workflow and issue creation, see `CLAUDE.md`.
