# Boot Deadlock Root Cause Analysis

**Project:** ZeroMount v2.0.0-dev (metamodule-experiment)
**Date:** 2026-02-10
**Method:** 16-agent paired investigation across 8 domains
**Reference:** mountify, meta-hybrid_mount, METAMODULE_COMPLETE_GUIDE.md

---

## Executive Summary

The boot deadlock is caused by two reinforcing mechanisms: (1) `service.sh` starts an **infinite watcher loop** (`watcher.rs:176`) that blocks forever under the `flock` instance lock, preventing `service.sh` from ever returning control to the Android init system, and (2) the bootloop protector is **non-functional** due to 4 interlocking bugs (`pipeline.rs:527-536`, `config.rs:345`) that create an infinite 3-boot reset cycle instead of disabling the module. Reference projects (mountify, meta-hybrid_mount) run their mount pipeline once and exit immediately -- none use a persistent watcher daemon during boot. The VFS/SUSFS layer was investigated and **ruled out** as a deadlock source; all kernel calls return immediately.

---

## Root Causes (ranked by evidence strength)

### RC1: Persistent Watcher Daemon Blocks service.sh Forever

**Evidence:** `cli/handlers.rs:35-46` calls `start_module_watcher()` which enters `watcher.rs:173-201` (`run_loop()`), an infinite `loop` with 10-second poll intervals. This call happens inside `handle_mount(post_boot=true)`, which is the code path invoked by `service.sh:16` (`"$BIN" mount --post-boot`). The watcher never returns -- `service.sh` blocks indefinitely.

**Domains confirming:** D1 (shell), D2 (mount), D7 (detection/watcher), D8 (timing)

**Reference behavior:** Both mountify and meta-hybrid_mount run their pipeline once at late_start and exit. Neither implements a persistent watcher daemon. Module hot-reload (if needed) is handled by KSU's own `metainstall.sh` callback, not a resident process.

**Impact:** CRITICAL. The Android init system expects `service.sh` to complete. A non-returning script can delay or block `sys.boot_completed`, downstream service startup, and may trigger Android's own watchdog reboot on some OEMs.

### RC2: Bootloop Protector Creates Infinite Reset Cycle

**Evidence:** Four confirmed bugs documented in `KNOWN-ISSUES.md` issue 8:

- **8a** (`pipeline.rs:527-536`): `run_pipeline_with_bootloop_guard()` detects bootloop (`check_bootloop()` returns true), restores backup config, then calls `run_full_pipeline(restored)` anyway. If the pipeline or its effects cause the loop, swapping config does nothing.
- **8b** (`config.rs:337-348`, specifically line 345): `restore_backup()` calls `Self::reset_bootcount()?` which resets the counter to 0. This creates an infinite cycle: count reaches 3 -> restore resets to 0 -> pipeline runs -> crash -> reboot -> count 1 -> 2 -> 3 -> reset to 0 -> repeat.
- **8c** (`service.sh:16` vs `metamount.sh:17-18`): `metamount.sh` has a shell-level bootcount guard (`[ "$COUNT" -ge 3 ] && exit 1`) but `service.sh` has no such guard. Even if metamount.sh bails, service.sh runs the pipeline unconditionally.
- **8d** (`handlers.rs:38`): Watcher callbacks invoke `run_pipeline_with_bootloop_guard()` for hot-reload, causing bootcount increments during normal runtime operation. This conflates runtime re-scans with boot failure tracking.

**Domains confirming:** D4 (pipeline -- root cause analysis), D1 (shell), D3 (install), D5 (spec)

**Reference behavior:** Mountify uses a threshold of 1 (not 3). On bootloop detection it disables the module entirely and exits. It does not attempt to swap configs and retry. The reset happens only after a fully successful boot.

**Impact:** CRITICAL. Even if RC1 were fixed, a bad module (e.g., KNOWN-ISSUES issue 7's font crash) would reboot the device forever because the protector resets its own counter before running the pipeline again.

### RC3: Double Pipeline Execution with Bootcount Double-Increment

**Evidence:** Each boot runs the pipeline twice:
1. `metamount.sh` at post-fs-data calls `zeromount mount` -> `handle_mount(false)` -> `run_pipeline_with_bootloop_guard()` at `handlers.rs:54`
2. `service.sh` at late_start calls `zeromount mount --post-boot` -> `handle_mount(true)` -> `run_pipeline_with_bootloop_guard()` at `handlers.rs:25`

Each call increments the bootcount via `ZeroMountConfig::increment_bootcount()` at `pipeline.rs:534`. Two increments per boot means the threshold (3) is hit after 1.5 boots, not 3 boots.

**Domains confirming:** D1 (shell), D4 (pipeline), D5 (spec), D8 (timing)

**Impact:** HIGH. Accelerates false-positive bootloop detection, which then triggers RC2's infinite reset cycle.

### RC4: Try-Lock Silently Drops service.sh on Contention

**Evidence:** `lock.rs:8-25` uses `LOCK_NB` (non-blocking flock). If `metamount.sh`'s pipeline is still running when `service.sh` starts, `acquire_instance_lock()` returns `None` and `handle_mount()` at `handlers.rs:11-14` logs a warning and returns `Ok(())`. The entire service.sh pipeline is silently skipped.

**Domains confirming:** D7 (detection/watcher), D2 (mount)

**Impact:** HIGH. On devices where post-fs-data takes longer (slow NAND, large modules), the main pipeline may never run because the lock contention window is non-deterministic. No retry mechanism exists.

### RC5: Unbounded Subprocess Calls Under flock Hold

**Evidence:** While the instance lock is held (for the entire lifetime of service.sh, which is forever due to the watcher):
- `platform.rs:37-38`: `Command::new("ksud").args(["module", "config", "set", ...])` -- no timeout
- `platform.rs:49-52`: `Command::new("ksud").args(["kernel", "notify-module-mounted"])` -- no timeout
- `storage.rs:615-651`: `nuke_ext4_sysfs()` calls `Command::new("ksud")` and `Command::new("insmod")` -- no timeout
- `storage.rs:693`: `fs::read_to_string("/proc/kallsyms")` -- full file read, blocking

The `storage.rs` functions do use `run_command_with_timeout()` for `dd`, `mkfs.erofs`, and `mkfs.ext4` (30s timeout at line 18), but the `ksud` and `insmod` calls in `nuke_ext4_sysfs()` bypass this and call `Command::output()` directly.

**Domains confirming:** D7 (detection/watcher), D8 (timing)

**Impact:** MEDIUM. If ksud hangs (daemon not ready at early boot), the pipeline hangs under flock, and any subsequent zeromount invocation is silently dropped by the try-lock.

---

## All Deviations from Reference Projects

### D1: Shell Scripts (9 deviations)

| Severity | Deviation | Our Behavior | Reference Behavior | Deadlock Risk |
|---|---|---|---|---|
| CRITICAL | Watcher infinite loop in service.sh | `handlers.rs:35-46` starts `run_loop()` (infinite) | Run once, exit | **Direct cause** |
| CRITICAL | Double pipeline with bootloop guard | Both metamount.sh and service.sh call `run_pipeline_with_bootloop_guard` | Single pipeline at late_start only | Doubles bootcount |
| HIGH | No bootcount guard in service.sh | `service.sh:16` calls binary unconditionally | N/A (single pipeline) | Bypasses protection |
| HIGH | post-fs-data runs detect, not mount | `post-fs-data.sh:18` runs `zeromount detect` | References skip post-fs-data or do minimal work | Wastes boot time |
| MEDIUM | metainstall.sh runs scanner binary | `metainstall.sh:21` runs `zeromount module scan` at install | References rely on watcher/next-boot | May hang install |
| MEDIUM | No partition symlink normalization | `handle_partition()` is a no-op at `metainstall.sh:11` | mountify normalizes partition symlinks | Cosmetic |
| MEDIUM | No single-instance lock at shell level | Lock only in Rust (`lock.rs`) | Some references use shell-level flock | Race window |
| LOW | No shell-level notify-module-mounted fallback | Rust calls it (`pipeline.rs:318`), no shell fallback | metamount.sh calls ksud as fallback | Minor if Rust crashes |
| LOW | service.sh exits 0 on all failures | `exit 0` at line 9 on unsupported arch | Some references exit 1 on failure | Masks errors |

### D2: Mount Pipeline (8 deviations)

| Severity | Deviation | Our Behavior | Reference Behavior | Deadlock Risk |
|---|---|---|---|---|
| CRITICAL | service.sh re-runs full pipeline | `handlers.rs:25` runs full pipeline at post-boot | Pipeline runs once | Wasted work, bootcount |
| CRITICAL | Watcher blocks service.sh forever | `watcher.rs:176` infinite loop | No watcher | **Direct cause** |
| HIGH | No shell fallback for notify-module-mounted | Only called from Rust finalize (`pipeline.rs:318`) | Shell fallback if binary fails | KSU may not register mount |
| HIGH | No storage sync phase | Files copied inline during mount execution | References pre-stage content | Slower, riskier |
| MEDIUM | No mount propagation control | Missing `MS_PRIVATE` before overlay | mountify uses `mount --make-private` | Mount leak possible |
| MEDIUM | No SELinux context on ext4 image | `storage.rs:493-551` mounts ext4 without SELinux context | meta-hybrid sets `context=u:object_r:...` | SELinux denial risk |
| MEDIUM | nuke_ext4_sysfs calls lack timeout | `storage.rs:615-651` uses raw `Command::output()` | N/A (references don't use ext4 loop) | Hang risk |
| LOW | Different module sort order | Alphabetical by default | Some references sort by priority | Cosmetic |

### D3: Module Install (6 deviations)

| Severity | Deviation | Our Behavior | Reference Behavior | Deadlock Risk |
|---|---|---|---|---|
| HIGH | Scanner runs at install time | `metainstall.sh:21` runs `zeromount module scan` | References do not run scanner at install | May hang install UI |
| HIGH | Watcher uses bootloop guard for hot-reload | `handlers.rs:38` calls `run_pipeline_with_bootloop_guard` | No bootloop guard for runtime reloads | Bootcount pollution |
| MEDIUM | handle_partition() is a no-op | `metainstall.sh:11` | mountify normalizes partitions | Missing feature |
| MEDIUM | Watcher debounce too short (2s) | `watcher.rs:30` `DEBOUNCE_MS: u64 = 2000` | Module installs take 5-10s | Premature re-scan |
| MEDIUM | Bootloop protector non-functional | KNOWN-ISSUES #8, 4 bugs confirmed | mountify: threshold=1, disable, exit | No safety net |
| LOW | No partition layout normalization | Not implemented | mountify creates partition symlinks | Minor |

### D4: Core Pipeline (root cause confirmed)

| Severity | Deviation | Our Behavior | Reference Behavior | Deadlock Risk |
|---|---|---|---|---|
| CRITICAL | Pipeline runs after bootloop detection | `pipeline.rs:530-531` restores config then runs anyway | Disable module, exit | **Infinite cycle** |
| CRITICAL | restore_backup() resets bootcount | `config.rs:345` calls `reset_bootcount()` | Only reset after successful boot | **Infinite cycle** |
| HIGH | service.sh has no shell guard | `service.sh` lacks `[ "$COUNT" -ge 3 ] && exit 1` | N/A (single entry point) | Bypasses protection |
| HIGH | Watcher increments bootcount at runtime | `handlers.rs:38` -> `pipeline.rs:534` | No bootcount for runtime ops | False positives |

### D7: Detection/Watcher (4 deviations)

| Severity | Deviation | Our Behavior | Reference Behavior | Deadlock Risk |
|---|---|---|---|---|
| CRITICAL | Persistent watcher daemon | `watcher.rs:173-201` infinite loop | Run once and exit | **Direct cause** |
| CRITICAL | Unbounded subprocess under flock | `platform.rs:37,49,97` ksud calls without timeout | N/A (no long-running lock) | Hang risk |
| HIGH | Try-lock silently drops service.sh | `lock.rs:14` `LOCK_NB` returns None on contention | N/A (no competing instances) | Silent skip |
| MEDIUM | Watcher re-runs with bootloop guard | `handlers.rs:38` | No bootcount for runtime ops | Bootcount pollution |

### D8: Boot Timing (6 blocking vectors)

| Severity | Blocking Vector | Location | Impact |
|---|---|---|---|
| CRITICAL | Watcher infinite loop | `watcher.rs:176` | service.sh never returns |
| HIGH | Triple detection execution | post-fs-data + metamount + service | ~3x detection overhead |
| HIGH | ksud calls without timeout | `platform.rs:37,49,97` | Blocks if ksud not ready |
| MEDIUM | /proc/kallsyms full read | `storage.rs:693` | ~50-200ms blocking I/O |
| MEDIUM | Watcher 2s debounce too short | `watcher.rs:30` | Premature re-scan during install |
| LOW | Double SUSFS pass | BRENE runs twice (post-fs-data + post-boot) | ~70ms wasted |

---

## Spec Violations (METAMODULE_COMPLETE_GUIDE.md)

| ID | Requirement | Our Status | Impact |
|---|---|---|---|
| V1 | metamount.sh must call `ksud kernel notify-module-mounted` | Called from Rust only (`pipeline.rs:318`), no shell fallback | KSU may not register mount if binary crashes before finalize |
| V2 | Binary exit code must be checked, fallback notification on failure | `metamount.sh:20` does not check exit code | Silent failure |
| V3 | Bootloop guard must notify KSU before exiting | `metamount.sh:18` does `exit 1` without calling ksud | KSU left in unknown state |
| V4 | post-fs-data should be lightweight (<2s) | `post-fs-data.sh:18` runs full detect phase | May exceed 10s KSU timeout |
| V5 | service.sh should run pipeline once and exit | `service.sh:16` runs pipeline + persistent watcher | **Direct deadlock cause** |
| V6 | Module ID should follow `meta-` prefix recommendation | ID is `zeromount`, not `meta-zeromount` | KSU may not prioritize correctly |
| V7 | metainstall.sh should create partition symlinks | `handle_partition()` is a no-op (`metainstall.sh:11`) | Module layout not normalized |
| V8 | metauninstall.sh should use `$MODULE_ID` variable | Uses `$1` positional arg | May break on future KSU changes |
| V9 | Declare `manage.kernel_umount` if handling umount | Not declared | KSU may conflict with umount handling |

---

## Ruled Out

### VFS/SUSFS Layer (D6)

The entire VFS and SUSFS subsystem was investigated and **eliminated as a deadlock source**:

- All SUSFS supercalls (`add_sus_path`, `add_sus_map`, `add_sus_kstat_redirect`) return immediately -- they are single `ioctl()` or `prctl()` calls to the kernel
- All error paths are handled gracefully: no retries, no loops, no blocking fallbacks
- The 412 ENOENT failures from KNOWN-ISSUES #4 add only ~20-40ms total (each `prctl` returns immediately with -2)
- The VFS driver `ioctl()` calls in `vfs/ioctls.rs` follow standard Linux semantics and cannot block indefinitely
- Double SUSFS pass (post-fs-data + post-boot) adds ~70ms worst case -- negligible
- BRENE EINVAL (-22) on re-runs (KNOWN-ISSUES #1) returns immediately, not a deadlock vector

---

## Cross-Domain Findings

These findings were only possible through multi-agent collaboration:

1. **4 agents independently corrected mount-2's finding #3:** `notify-module-mounted` IS called from Rust `finalize()` at `pipeline.rs:318`, not missing entirely. The real risk is the pipeline hanging BEFORE reaching finalize (due to RC1/RC5), in which case the notification never fires and there is no shell-level fallback.

2. **install-2 discovered `--update-conf` may be a no-op:** `metainstall.sh:21` calls `zeromount module scan --update-conf`, but `handlers.rs:193-194` shows the `update_conf` flag only logs a debug message (`"partitions.conf rebuild requested"`) and does nothing. The scan output is printed to stdout but not persisted.

3. **pipeline-2 corrected mountify's bootloop threshold:** Mountify uses threshold=1 (not 2 as initially reported). This means mountify disables the module after the FIRST failed boot, while ZeroMount waits for 3 (and then resets to 0 due to RC2).

4. **D4+D1 together revealed the bootcount arithmetic bug:** Two increments per boot (metamount.sh + service.sh) means threshold 3 is hit after 1.5 boots, not 3. Combined with RC2's reset-to-0, this creates a cycle of: 2 increments -> 2 more -> hits 3 -> reset to 0 -> repeat forever.

5. **D7+D8 together identified the flock deadlock amplifier:** The watcher holds the instance lock forever (RC1). Any ksud call that hangs under this lock (RC5) means the lock is never released, and any concurrent zeromount invocation is silently dropped (RC4).

---

## Fix Priority Order

1. **Remove the persistent watcher from service.sh path** -- Make `handle_mount(post_boot=true)` run the pipeline once and return, like references do. Module hot-reload can be deferred to metainstall.sh callbacks or a separate daemon not blocking boot. Effort: small (remove `start_module_watcher` call from handlers.rs:35-46).

2. **Fix bootloop protector (4 bugs):**
   - 2a: When bootloop detected, skip pipeline entirely -- return a safe-mode RuntimeState with zero rules instead of calling `run_full_pipeline()`.
   - 2b: Remove `reset_bootcount()` from `restore_backup()` (`config.rs:345`). Only reset in `finalize()` after successful pipeline completion.
   - 2c: Add shell guard to `service.sh` matching `metamount.sh:17-18`.
   - 2d: Watcher callbacks (if kept) should call `run_full_pipeline()` directly, not `run_pipeline_with_bootloop_guard()`.
   Effort: moderate (logic changes in pipeline.rs, config.rs, service.sh, handlers.rs).

3. **Add timeouts to all subprocess calls** -- Wrap `platform.rs` ksud calls and `storage.rs:615-651` nuke calls with `run_command_with_timeout()` (already implemented in storage.rs:24-59). Effort: small.

4. **Eliminate double pipeline execution** -- Either skip the metamount.sh pipeline entirely (let service.sh handle everything, matching KNOWN-ISSUES #6 recommendation) or make service.sh only do post-mount tasks (UID blocking, WebUI) without re-running the pipeline. Effort: small.

5. **Add shell-level fallbacks for notify-module-mounted** -- In metamount.sh, check binary exit code and call `ksud kernel notify-module-mounted` from shell if the binary fails. Effort: small.

6. **Convert try-lock to blocking lock with timeout** -- Replace `LOCK_NB` in `lock.rs` with a blocking flock that times out after ~30s, so service.sh waits for metamount.sh to finish rather than silently skipping. Effort: small.

7. **Address remaining spec violations** -- V6 (module ID prefix), V7 (partition symlinks), V8 ($MODULE_ID), V9 (manage.kernel_umount). Effort: small per item.

---

## Appendix: Deadlock Scenario (step-by-step)

The definitive boot sequence showing where and why the device hangs:

```
BOOT START
  |
  v
[post-fs-data] KSU triggers post-fs-data.sh
  |-- zeromount detect (post-fs-data.sh:18)
  |   Probes kernel capabilities, writes .detection.json
  |   Duration: <2s normally
  |
  v
[post-fs-data] KSU triggers metamount.sh
  |-- Read .bootcount (metamount.sh:17)
  |-- Shell guard: if count >= 3, exit 1 (metamount.sh:18)
  |   BUT: config.rs:345 reset bootcount on previous boot -> count is always 0 or low
  |-- acquire_instance_lock() (handlers.rs:9-15) -- succeeds (first invocation)
  |-- run_pipeline_with_bootloop_guard() (handlers.rs:54)
  |   |-- increment_bootcount() (pipeline.rs:534) -- count becomes 1
  |   |-- run_full_pipeline() -> detect -> scan (4 files visible) -> execute -> finalize
  |   |-- finalize: notify-module-mounted (pipeline.rs:318)
  |   |-- finalize: reset_bootcount() (pipeline.rs:313) -- count back to 0
  |-- Lock released, zeromount exits
  |
  v
[late_start] Android init triggers service.sh
  |-- zeromount mount --post-boot (service.sh:16)
  |-- acquire_instance_lock() (handlers.rs:9-15) -- succeeds (metamount finished)
  |-- run_pipeline_with_bootloop_guard() AGAIN (handlers.rs:25)
  |   |-- increment_bootcount() (pipeline.rs:534) -- count becomes 1 again
  |   |-- run_full_pipeline() -> detect -> scan (226 files now) -> execute -> finalize
  |   |-- finalize: reset_bootcount() -- count back to 0
  |
  |-- >>> start_module_watcher() (handlers.rs:35-46) <<<
  |   |-- ModuleWatcher::new() -> inotify_init1 (watcher.rs:55)
  |   |-- watcher.run_loop() (watcher.rs:173)
  |   |   |
  |   |   |   loop {  <-- INFINITE LOOP, NEVER RETURNS
  |   |   |       poll(10_000ms)  -- wait for inotify events
  |   |   |       if events -> debounce(2000ms) -> on_change()
  |   |   |       on_change() calls run_pipeline_with_bootloop_guard() AGAIN
  |   |   |           -> increments bootcount (stays under threshold due to resets)
  |   |   |       continue  -- back to poll
  |   |   |   }
  |   |   |
  |   |   v
  |   |   NEVER REACHES HERE
  |   |
  |   v
  |-- handle_mount() NEVER RETURNS
  |
  v
service.sh HANGS FOREVER -- zeromount binary never exits
  |
  v
Android init: service.sh marked as "running" indefinitely
sys.boot_completed may or may not be set (depends on OEM)
If a bad module causes Zygote crash (KNOWN-ISSUES #7):
  -> Zygote restarts in loop
  -> sys.boot_completed never becomes 1
  -> Bootloop protector cannot help (4 bugs from RC2)
  -> Device stuck on boot logo until battery dies or ADB rescue
```

**The fix is straightforward:** Remove `start_module_watcher()` from the `post_boot=true` path in `handlers.rs:35-46`. Run the pipeline once and return, exactly like reference projects do.
