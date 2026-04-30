# Learning Workflow

## Inverted Model

In this project, the user designs and implements. Claude validates and supports.

```
User designs  →  Claude validates (/validate-design)
User codes    →  Claude reviews (ad-hoc or /review on PR)
User is stuck →  Claude explains, gives hints
Code is ready →  Claude writes docs (/document), user commits
```

## Issue Workflow

### 0. Feynman checkpoint

Before coding, the user explains the key Rust concepts for this issue in their own
words. Claude challenges the understanding — not the code.

### 1. User presents their design

The user describes their approach to an issue. Claude:

- Reads the issue for context
- Challenges the design (see `/validate-design`)
- **Does NOT propose an alternative design**

### 2. User implements

The user writes the code. Claude:

- Answers questions about Rust concepts
- Reviews code snippets when asked
- **Does NOT write implementation code**

### 3. User is stuck

When the user hits a wall, Claude escalates gradually:

1. Explain the concept
2. Give a targeted hint
3. Show a minimal example (5-10 lines, generic — not the user's code)
4. Only write the actual solution if explicitly asked

### 4. Code is ready

When the user's code is working:

- Claude reviews the PR (`/review`)
- Claude writes documentation (`/document`)
- User commits (`/commit`)
- User creates PR (`/create-pr`)
