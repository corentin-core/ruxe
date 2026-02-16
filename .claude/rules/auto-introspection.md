# Auto-Introspection

## Trigger

This rule activates **automatically** when the user:

- Contradicts you ("no", "that's wrong", "you're mistaken")
- Points out an error ("you made a mistake", "that's not it", "bad analysis")
- Corrects your work ("there's no point in...", "you should have...")
- Expresses frustration with your output ("this is bad", "start over")

## Response Pattern

When triggered, **always** end your response with a proposal to update your config:

```markdown
---

**Improvement proposal:**

I made the mistake of [concise description]. To avoid this in the future:

- **File to modify**: `.claude/rules/X.md` | `.claude/skills/X/SKILL.md` |
  `CLAUDE.md`
- **Proposed change**: [description of the change]

Should I apply this change?
```

## Decision Tree

```
User points out error
  ↓
Is it a recurring pattern that could happen again?
  YES → Propose rule/skill update
  NO  → Just acknowledge and fix, no config change needed

What type of mistake?
  - Wrong process/workflow → Update skill (SKILL.md)
  - Missing knowledge about project → Update CLAUDE.md
  - Bad habit/pattern → Create/update rule (.claude/rules/)
```

## Important

- **Don't wait to be asked** - propose the improvement proactively
- **Be specific** - identify the exact file and change needed
- **Be concise** - the proposal should be 3-5 lines max
- **Ask permission** - never modify config without user approval