# Design

> **Purpose:** Your blueprint. Prevents "figure it out as I go."
> **Target length:** 1-2 pages. If longer, split into component docs.

---

## Approach

<!-- 1 paragraph explaining the strategy. Why this approach? -->

[TODO: How will you solve the problem outlined in GOAL.md?]

---

## Components

### Component A: [Name]

- **Purpose:** What it does
- **Inputs:** What it receives
- **Outputs:** What it produces
- **Dependencies:** What it needs

### Component B: [Name]

- **Purpose:** What it does
- **Inputs:** What it receives
- **Outputs:** What it produces
- **Dependencies:** What it needs

### Component C: [Name]

- **Purpose:** What it does
- **Inputs:** What it receives
- **Outputs:** What it produces
- **Dependencies:** What it needs

---

## Data Flow

<!-- How do components connect? -->

```
[Input] ──► [Component A] ──► [Component B] ──► [Output]
                 │
                 ▼
            [Component C]
```

---

## Error Handling

<!-- What happens when things go wrong? -->

| Failure Scenario | Behavior | Recovery |
|------------------|----------|----------|
| [X fails] | [What happens] | [How to recover] |
| [Y unavailable] | [What happens] | [How to recover] |

---

## File Structure

<!-- Where does code live? -->

```
src/
├── [folder]/
│   ├── [file.ext]    # [purpose]
│   └── [file.ext]    # [purpose]
└── [folder]/
    └── [file.ext]    # [purpose]
```

---

## API/Interface (If Applicable)

<!-- How do external things interact with your code? -->

```
[Function/endpoint signature]
  Input: [what]
  Output: [what]
  Side effects: [what]
```

---

## Example (Delete This Section)

```markdown
## Approach
Intercept metamodule's mounting logic and redirect to VFS layer instead of
creating actual bind mounts. This makes module files accessible without
leaving traces in /proc/mounts.

## Components
### VFS Redirector
- Purpose: Intercept file access and redirect to module paths
- Inputs: File access requests
- Outputs: Redirected file handles
- Dependencies: SUSFS kernel support

### Mount Interceptor
- Purpose: Catch mount requests before they execute
- Inputs: Mount syscall arguments
- Outputs: Blocked or modified mount
- Dependencies: KernelSU hooks
```
