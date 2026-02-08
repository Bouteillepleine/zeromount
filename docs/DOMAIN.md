# Domain Knowledge

> **Purpose:** Prevents building on sand. The documentation you MUST read first.
> **Target length:** 1-2 pages. Link to external docs, don't copy them.

---

## Required Reading (In Order)

<!-- List the documentation/resources you MUST understand before building -->

1. [Link] - [What it covers]
2. [Link] - [What it covers]
3. [Link] - [What it covers]

**Status:** [ ] Not started  [ ] In progress  [ ] Completed

---

## Key Concepts

<!-- Terms you need to understand. Explain in YOUR words. -->

| Concept | Definition | Why It Matters |
|---------|------------|----------------|
| [Term 1] | [What it is] | [How it affects your project] |
| [Term 2] | [What it is] | [How it affects your project] |
| [Term 3] | [What it is] | [How it affects your project] |

---

## System Behavior

<!-- How does the system you're building on actually work? -->
<!-- Use diagrams, sequences, or bullet points -->

```
[Diagram or sequence of how the system operates]
```

---

## Gotchas & Common Mistakes

<!-- What trips people up? What did YOU get wrong initially? -->

| Mistake | Why It's Wrong | Correct Approach |
|---------|----------------|------------------|
| [Mistake 1] | [Explanation] | [What to do instead] |
| [Mistake 2] | [Explanation] | [What to do instead] |

---

## Questions Still Unanswered

<!-- What don't you understand yet? Track these. -->

- [ ] [Question 1]
- [ ] [Question 2]

---

## Example (Delete This Section)

```markdown
## Required Reading
1. https://kernelsu.org/guide/module.html - KernelSU module structure
2. SUSFS documentation - How path hiding works
3. Linux VFS documentation - Virtual filesystem layer basics

## Key Concepts
| Concept | Definition | Why It Matters |
|---------|------------|----------------|
| Metamodule | Module that manages other modules | We hook into its mounting process |
| skip_mount | Flag that prevents metamodule mounting | Misusing this breaks everything |
| VFS | Virtual File System layer | Our redirection happens here |

## Gotchas
| Mistake | Why It's Wrong | Correct Approach |
|---------|----------------|------------------|
| Using skip_mount everywhere | Disables metamodule for that module | Only use for modules that self-mount |
```
