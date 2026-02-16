# Handle PR Comments

Address review comments on a GitHub pull request interactively.

## Arguments

- `$ARGUMENTS`: PR number or URL (e.g., `200` or
  `https://github.com/user/repo/pull/200`)

## Workflow

### Phase 1: Collect and Analyze

1. **Extract PR number** from arguments (strip URL if needed)

2. **Fetch unresolved review comments**:

```bash
gh api repos/{owner}/{repo}/pulls/<number>/comments \
  --jq '.[] | {id, user: .user.login, path, line, body}'

gh api repos/{owner}/{repo}/issues/<number>/comments \
  --jq '.[] | {id, user: .user.login, body}'
```

3. **Present a summary** of all comments to the user

### Phase 2: Process Each Comment

For each comment, **one at a time**:

1. **Read the relevant code** to understand the context
2. **Propose a solution**
3. **Wait for user validation**
4. **Apply the change** if approved
5. Move to the next comment

### Phase 3: Commit and Push

After all comments are addressed:

1. **Run tests**: `cargo test && cargo clippy -- -D warnings`
2. **Commit** with a descriptive message
3. **Push** to the remote branch

### Phase 4: Reply to Comments

For each comment that was addressed:

```bash
gh api repos/{owner}/{repo}/pulls/<number>/comments/<comment_id>/replies \
  -X POST \
  -f body="🤖 Claude: <response>"
```

### Phase 5: Introspection

After all comments are replied to, reflect on patterns and propose rule updates if
relevant.

## Response Guidelines

- **Be concise**: explain what was done
- **Reference commits**: include commit hash
- **Prefix with**: `🤖 Claude:`

## Key Principles

1. **One comment at a time** - Don't batch process
2. **Propose before acting** - Always get approval
3. **Test before commit** - Run tests after all changes
4. **Validate before posting** - Let user approve each reply
5. **Learn from feedback** - Update rules to prevent future similar comments

$ARGUMENTS