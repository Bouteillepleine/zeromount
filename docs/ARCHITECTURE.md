# Architecture

> **Purpose:** Forces you to understand before building. The diagram you draw on paper.
> **Target length:** 1-2 pages. Use diagrams, not paragraphs.

---

## System Overview

<!-- One diagram showing major components and data flow -->
<!-- ASCII art, Mermaid, or just describe the boxes and arrows -->

```
┌─────────────┐     ┌─────────────┐     ┌─────────────┐
│ Component A │ ──► │ Component B │ ──► │ Component C │
└─────────────┘     └─────────────┘     └─────────────┘
       │                                       │
       └───────────────────────────────────────┘
```

---

## Boot/Execution Sequence

<!-- When does your code run? What happens before and after? -->

```
Step 1: [System does X]
Step 2: [System does Y]
   └── YOUR CODE RUNS HERE
Step 3: [System does Z]
```

---

## Key Components

<!-- What are the major pieces of the system? -->

| Component | Responsibility | Location |
|-----------|---------------|----------|
| [Thing A] | [Does X] | [/path/to/] |
| [Thing B] | [Does Y] | [/path/to/] |
| [Thing C] | [Does Z] | [/path/to/] |

---

## Integration Points

<!-- Where does your code hook into the system? -->

- We hook into [X] at [point Y]
- We depend on [Z] being available
- We must run before/after [W]

---

## Data Flow

<!-- How does data move through the system? -->

```
Input: [What triggers the flow]
   │
   ▼
[Process 1]
   │
   ▼
[Process 2]
   │
   ▼
Output: [What gets produced]
```

---

## Constraints

<!-- Technical limitations you must work within -->

- Must complete in <[N] seconds
- Cannot use [X] because [Y]
- Must be compatible with [Z]
- Memory limit: [X]
- Dependencies: [list]

---

## Example (Delete This Section)

```markdown
## System Overview
┌──────────────┐     ┌─────────────┐     ┌────────────┐
│   KernelSU   │ ──► │ Metamodule  │ ──► │  Modules   │
│   (ksud)     │     │  (mounts)   │     │ (payloads) │
└──────────────┘     └─────────────┘     └────────────┘
                            │
                     ┌──────┴──────┐
                     │  OUR CODE   │
                     │ (intercept) │
                     └─────────────┘

## Boot Sequence
1. Kernel boots
2. init starts
3. KernelSU ksud starts
4. Metamodule triggered
   └── OUR HOOK intercepts here
5. Modules mounted (or VFS redirected)
6. System continues boot
```
