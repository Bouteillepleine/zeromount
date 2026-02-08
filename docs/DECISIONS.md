# Decisions Log

> **Purpose:** Captures reasoning. Prevents re-debating solved problems.
> **Target length:** Half page per decision. This file grows over time.

---

## Decision Template

Copy this for each new decision:

```markdown
## Decision N: [Short Title]

**Date:** YYYY-MM-DD
**Status:** Accepted / Superseded / Deprecated

**Context:**
What situation required a decision?

**Options Considered:**
1. **Option A** - [brief description]
   - Pros: [list]
   - Cons: [list]

2. **Option B** - [brief description]
   - Pros: [list]
   - Cons: [list]

3. **Option C** - [brief description]
   - Pros: [list]
   - Cons: [list]

**Decision:**
We chose [Option X] because [reasoning].

**Consequences:**
- We gain: [benefits]
- We lose: [tradeoffs]
- We must now: [follow-up actions]
```

---

## Decisions

<!-- Add decisions below, newest first -->

### Decision 1: [Title]

**Date:** [Date]
**Status:** Accepted

**Context:**
[What situation required a decision?]

**Options Considered:**
1. **Option A**
   - Pros:
   - Cons:

2. **Option B**
   - Pros:
   - Cons:

**Decision:**
[What you chose and why]

**Consequences:**
- We gain:
- We lose:
- We must now:

---

## Example (Delete This Section)

```markdown
### Decision 1: Mount Interception Method

**Date:** 2026-01-27
**Status:** Accepted

**Context:**
Need to prevent bind mounts from appearing in /proc/mounts while still
making module files accessible.

**Options Considered:**
1. **Patch /proc/mounts output**
   - Pros: Simple, doesn't change mount behavior
   - Cons: Race conditions, some tools read /proc/self/mountinfo

2. **VFS redirection instead of mounting**
   - Pros: No mounts to hide, architecturally clean
   - Cons: More complex, requires SUSFS support

3. **Unmount immediately after access**
   - Pros: Files accessible during use
   - Cons: Timing issues, detectable during mount window

**Decision:**
VFS redirection (Option 2) because it solves the root problem instead of
hiding symptoms. SUSFS support is available in our target kernels.

**Consequences:**
- We gain: Undetectable module access
- We lose: Compatibility with non-SUSFS kernels
- We must now: Implement VFS hooks, require SUSFS dependency
```
