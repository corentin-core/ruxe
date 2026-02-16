# Auto-Introspection

## Trigger

This rule activates **automatically** when the user:

- Contradicts you ("non", "c'est faux", "tu as tort")
- Points out an error ("tu as fait une erreur", "c'est pas ça", "mauvaise analyse")
- Corrects your work ("ça sert à rien de...", "il fallait plutôt...")
- Expresses frustration with your output ("c'est nul", "recommence")

## Response Pattern

When triggered, **always** end your response with a proposal to update your config:

```markdown
---

**Proposition d'amélioration :**

J'ai fait l'erreur de [description concise]. Pour éviter ça à l'avenir :

- **Fichier à modifier** : `.claude/rules/X.md` | `.claude/skills/X/SKILL.md` |
  `CLAUDE.md`
- **Changement proposé** : [description du changement]

Veux-tu que j'applique cette modification ?
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