# Goal

> **Purpose:** Prevents scope creep. Forces clarity. Fill this FIRST.
> **Target length:** Half page. If longer, you're overcomplicating.

---

## One-Sentence Summary

<!-- What does this project do in 15 words or less? -->

[TODO: Write one sentence]

---

## Success Criteria

<!-- How do you know when it's "done"? Make these measurable. -->

- [ ] [Criterion 1 - specific and testable]
- [ ] [Criterion 2 - specific and testable]
- [ ] [Criterion 3 - specific and testable]

---

## Explicitly Out of Scope

<!-- What are you NOT building? This prevents creep. -->

- NOT doing [X]
- NOT supporting [Y]
- NOT handling [Z]

---

## Why This Matters

<!-- 1-2 sentences on the problem being solved -->

[TODO: Why does this project need to exist?]

---

## Example (Delete This Section)

```markdown
## One-Sentence Summary
A KernelSU module that mounts modules via VFS redirection with zero detectable mounts.

## Success Criteria
- [ ] Modules function correctly (files accessible at expected paths)
- [ ] Zero entries in /proc/mounts from module files
- [ ] Momo/Shamiko detection passes on test device

## Explicitly Out of Scope
- NOT replacing SUSFS (we use it, not replace it)
- NOT supporting Magisk (KernelSU only for v1)
- NOT handling APatch compatibility

## Why This Matters
Overlay/bind mounts are detectable by banking apps. VFS redirection is invisible.
```
