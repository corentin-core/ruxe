---
name: commit
description: Create a well-formatted git commit following project conventions
---

# Commit Changes

Create a well-formatted git commit.

## Instructions

1. Check the current status and diff:

```bash
git status
git diff --staged
```

2. If nothing is staged, ask what should be committed

3. Run tests and clippy before committing:

```bash
cargo test
cargo clippy -- -D warnings
```

4. Write a commit message following conventional commits:

   - `feat:` new feature
   - `fix:` bug fix
   - `refactor:` code refactoring
   - `test:` adding tests
   - `docs:` documentation
   - `chore:` maintenance

5. Create the commit:

```bash
git commit -m "type: description"
```

**Note**: Do NOT add `Co-Authored-By` lines (per project conventions).

6. Show the result with `git log -1`

$ARGUMENTS
