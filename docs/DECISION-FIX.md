# Boot Deadlock Fix Decisions

**Source:** BOOT-DEADLOCK-ANALYSIS.md (16-agent investigation, 2026-02-10)
**Status:** DECISION PHASE — RC/V/D decisions finalized, WebUI decisions pending

---

## Reference Documentation

All decisions in this document were verified against these sources. A new session MUST read the relevant reference before implementing any decision.

### Authoritative KSU Specs (5 files, ~4,627 lines)

| Doc | Lines | Path | Covers |
|---|---|---|---|
| METAMODULE_COMPLETE_GUIDE.md | 838 | `/home/claudetest/gki-build/METAMODULE_COMPLETE_GUIDE.md` | Metamodule spec — boot flow, notify-module-mounted, meta- prefix, metainstall/metamount/metauninstall contracts |
| kernelsu-module-guide.md | 1140 | `/home/claudetest/zero-mount/reference/kernelsu-module-guide.md` | General module dev — scripts, boot stages, 10s post-fs-data timeout, skip_mount, disable, MODDIR |
| kernelsu-module-webui.md | 1087 | `/home/claudetest/zero-mount/reference/kernelsu-module-webui.md` | WebUI API — ksu.exec, spawn, listPackages, moduleInfo, enableInsets, kernelsu npm package |
| kernelsu-module-config.md | 502 | `/home/claudetest/zero-mount/reference/kernelsu-module-config.md` | Module config API — manage.kernel_umount, ksud module config set/get |
| kernelsu-additional-docs.md | 1060 | `/home/claudetest/zero-mount/reference/kernelsu-additional-docs.md` | Additional KSU documentation — supplementary APIs and behaviors |

### Reference Projects — Mountify (~3,702 lines)

Base path: `/home/claudetest/zero-mount/context-gathering/output/mountify/`

| File | Lines | Covers |
|---|---|---|
| `context-mountify.md` | 662 | Full architecture — boot flow, storage modes, overlay, SELinux, LKM nuke, single-instance lock |
| `parts/core-logic.md` | 575 | Mount execution, `cp -Lrf` file staging, overlay creation, SELinux `chcon --reference` |
| `parts/integration.md` | 614 | Shell scripts, boot sequence, install hooks, metainstall.sh (no scanner) |
| `parts/structure.md` | 390 | File layout, module.prop, directory structure |
| `parts/webui.md` | 572 | WebUI implementation |
| `validation-mountify.md` | 301 | Cross-validation findings |
| `validation-part-a.md` | 239 | Validation details A |
| `validation-part-b.md` | 349 | Validation details B |

### Reference Projects — Meta-hybrid_mount (~5,264 lines)

Base path: `/home/claudetest/zero-mount/context-gathering/output/meta-hybrid_mount/`

| File | Lines | Covers |
|---|---|---|
| `context-meta-hybrid_mount.md` | 985 | Full architecture — atomic sync, EROFS, MS_PRIVATE, reverse-alpha sort |
| `parts/core-logic.md` | 689 | Mount execution, sync.rs, atomic rename (.tmp/.backup pattern) |
| `parts/integration.md` | 733 | Shell scripts, boot sequence, install hooks, no service.sh (metamount.sh only) |
| `parts/structure.md` | 679 | File layout, Cargo workspace |
| `parts/webui.md` | 640 | WebUI implementation |
| `validation-meta-hybrid_mount.md` | 371 | Cross-validation findings |
| `validation-part-a.md` | 458 | Validation details A |
| `validation-part-b.md` | 709 | Validation details B |

### Our Generated Analysis (in-project, `docs/`)

| File | Lines | Covers |
|---|---|---|
| `DECISION-FIX.md` | this file | All fix decisions with evidence and implementation order |
| `BOOT-DEADLOCK-ANALYSIS.md` | 286 | 5 root causes, 41 deviations, 9 spec violations, step-by-step deadlock walkthrough |
| `KSU-COMPLIANCE-AUDIT.md` | 186 | 10-agent audit: 131 PASS, 10 FAIL (8 WebUI), 28 WARN across 5 domains |
| `KNOWN-ISSUES.md` | 152 | 9 known issues from live device testing (Redmi 14C, KSU Next, SUSFS v2.0.0) |

---

## RC1: Persistent Watcher Daemon Blocks service.sh Forever

**Severity:** CRITICAL
**Evidence:** `handlers.rs:35-46` calls `start_module_watcher()` which enters `watcher.rs:173-201` infinite loop. service.sh never returns.
**Reference behavior:** Both mountify and meta-hybrid_mount run pipeline once at late_start and exit. Neither uses a persistent watcher.
**Confirmed by:** D1, D2, D7, D8

**Decision:** OPTION A — Kill the watcher entirely. Remove `start_module_watcher()` call from `handlers.rs:35-46`. Pipeline runs once at post-boot, exits. Module hot-reload handled by KSU's `metainstall.sh` callback (already calls `zeromount module scan --update-conf`). Matches both reference projects. The watcher code in `watcher.rs` stays in the codebase (not dead — could be used for an explicit `zeromount watch` subcommand later) but is no longer called from the boot path.

**Files to change:** `src/cli/handlers.rs` (remove lines 34-46, the watcher block)

---

## RC2: Bootloop Protector Creates Infinite Reset Cycle

**Severity:** CRITICAL
**Evidence:** Four interlocking bugs:
- 8a: `pipeline.rs:527-536` — detects bootloop, restores config, runs pipeline anyway
- 8b: `config.rs:345` — `restore_backup()` resets bootcount to 0, creating infinite 3-boot cycle
- 8c: `service.sh` has no shell-level bootcount guard (metamount.sh:17-18 does)
- 8d: `handlers.rs:38` — watcher callbacks use bootloop guard, polluting bootcount at runtime
**Reference behavior:** Mountify threshold=1, disables module on detection, exits. Resets only after successful boot.
**Confirmed by:** D1, D3, D4, D5

**Decision:** Fix all 4 bugs:

- **8a:** On bootloop detection, return empty safe-mode `RuntimeState` (zero rules, no mounts). Do NOT run the pipeline. Call `notify-module-mounted` before returning so KSU's post-fs-data gate doesn't block.
- **8b:** Remove `reset_bootcount()` from `restore_backup()` (`config.rs:345`). Bootcount only resets in `finalize()` after a successful pipeline (`pipeline.rs:313`).
- **8c:** Add bootcount shell guard to `service.sh` matching `metamount.sh:17-18`. Defense in depth.
- **8d:** No action needed — goes away with RC1 fix (watcher killed). If hot-reload returns later, it must use `run_full_pipeline()` directly, not the bootloop-guarded version.

**Files to change:** `src/core/pipeline.rs` (rewrite `run_pipeline_with_bootloop_guard`), `src/core/config.rs` (remove line 345), `module/service.sh` (add 2-line guard)

---

## RC3: Double Pipeline Execution with Bootcount Double-Increment

**Severity:** HIGH
**Evidence:** metamount.sh runs `run_pipeline_with_bootloop_guard()` at post-fs-data, service.sh runs it again at late_start. Each increments bootcount via `pipeline.rs:534`. Threshold 3 hit after 1.5 boots, not 3.
**Confirmed by:** D1, D2, D3, D4, D8

**Decision:** OPTION B — Single pipeline at service.sh only. metamount.sh becomes a thin notify-and-exit: call `ksud kernel notify-module-mounted` from shell and return. The full pipeline runs once at late_start via service.sh when module files are fully extracted. Evidence: KNOWN-ISSUES #6 shows post-fs-data run finds 4/226 files — wasted work. The late_start run is always the authoritative one.

**Files to change:** `module/metamount.sh` (replace binary call with notify + exit), `src/cli/handlers.rs` (remove `post_boot=false` pipeline path or simplify)

---

## RC4: Try-Lock Silently Drops service.sh on Contention

**Severity:** HIGH
**Evidence:** `lock.rs:14` uses `LOCK_NB`. If metamount.sh is still running, `acquire_instance_lock()` returns None, `handlers.rs:11-14` logs warning and returns Ok(()). Entire service.sh pipeline silently skipped.
**Confirmed by:** D7, D8

**Decision:** OPTION A — Leave the try-lock as-is. The contention window (metamount.sh vs service.sh) is eliminated by RC3 (metamount.sh no longer invokes the binary's mount path). Try-lock still correctly prevents double-invocation from manual CLI use. No code changes needed.

**Files to change:** None

---

## RC5: Unbounded Subprocess Calls Under flock Hold

**Severity:** MEDIUM
**Evidence:**
- `platform.rs:37-38` — `ksud module config set` — no timeout
- `platform.rs:49-52` — `ksud kernel notify-module-mounted` — no timeout
- `storage.rs:615-651` — `nuke_ext4_sysfs()` ksud + insmod calls — no timeout
- `storage.rs:693` — `/proc/kallsyms` full read — blocking I/O
**Confirmed by:** D7, D8

**Decision:** Move `run_command_with_timeout` from `storage.rs:24-59` to `src/utils/command.rs` (new file). Apply it consistently to all subprocess calls: `platform.rs:37,49` (ksud calls), `storage.rs:615-651` (ksud + insmod in `nuke_ext4_sysfs`). Keep existing 30s timeout. No new logic — just consistent application of the existing pattern.

**Files to change:** `src/utils/command.rs` (new — extract from `storage.rs:24-59`), `src/utils/mod.rs` (add module), `src/mount/storage.rs` (import from utils instead of local), `src/utils/platform.rs` (wrap ksud calls)

---

## Spec Deviations

### V1-V3, V5: Resolved by RC1-RC3

V1 (notify-module-mounted from shell), V2 (check binary exit code), V3 (notify on bootloop bail-out), and V5 (service.sh run once and exit) are all addressed by RC1, RC2, and RC3 decisions above. No additional action needed.

### V4: post-fs-data should be lightweight

**Status:** Moot — resolved by RC3. metamount.sh no longer invokes the Rust binary's mount path. post-fs-data.sh runs `zeromount detect` only (kernel probes, <2s). No action needed.

### V6: Module ID should follow meta- prefix

**Spec:** METAMODULE_COMPLETE_GUIDE.md:170 — *"It is strongly recommended to name your metamodule ID starting with `meta-`"*
**Current:** `module.prop:1` has `id=zeromount`

**Decision:** Rename module ID from `zeromount` to `meta-zeromount`. Ripple through all references: `module.prop`, `constants.ts`, config paths, shell scripts referencing `/data/adb/modules/zeromount/`. Additional documentation sources pending review.

**Files to change:** TBD after full impact assessment

### V7: metainstall.sh partition symlinks — NO ACTION

**Current:** `metainstall.sh:11` overrides `handle_partition()` as no-op. Intentional per code comment.
**Verified:** No documentation exists for `handle_partition()` in any of the three supplementary KSU docs (`kernelsu-module-guide.md`, `kernelsu-module-config.md`, `kernelsu-additional-docs.md`). Partitions are pre-mounted by KSU before metamount.sh runs (`kernelsu-module-guide.md:391`). The no-op is correct — ZeroMount handles module overlay layout internally.

### V8: metauninstall.sh $MODULE_ID — DROPPED

**Not a violation.** The spec's own example (`METAMODULE_COMPLETE_GUIDE.md:293-294`) uses `MODULE_ID="$1"`. Our code uses `$1` directly — matches the spec's pattern.

### V9: manage.kernel_umount declaration

**Verified:** Documented in `kernelsu-module-config.md:1069-1070`. Modules declare ownership via `ksud module config set manage.kernel_umount false`. Value `false` disables KSU's automatic kernel-level umount for this module.
**Relevance:** ZeroMount does its own `cleanup_previous_mounts()` at `pipeline.rs:444-475`. For overlay/magic fallback paths, ZeroMount creates mounts with `source=KSU` that KSU's kernel_umount would also try to manage.

**Decision:** Declare `manage.kernel_umount false` during module installation (`customize.sh`). This tells KSU that ZeroMount owns umount lifecycle — prevents conflicts on overlay/magic fallback paths.

**Files to change:** `module/customize.sh` (add `ksud module config set manage.kernel_umount false`)

---

## Reference Alignment Deviations (41 total, 33 resolved by RC1-RC5/V1-V9 above)

The 16-agent investigation cataloged 41 deviations from reference projects (mountify, meta-hybrid_mount) across 7 domains. 33 of those are already resolved by the RC1-RC5 and V1-V9 decisions above. The remaining 8 are decided below.

**Cross-reference:** For the full 33→decision mapping, see the triage table in the session transcript. Every deviation from BOOT-DEADLOCK-ANALYSIS.md sections D1-D8 has been accounted for.

---

### D2-4: Two-phase overlay staging with atomic rename

**Severity:** HIGH (reliability)
**What it is:** When mounting overlayfs, module files must be copied into "lower dirs" before the overlay can be created. Currently, `executor.rs:31-84` interleaves file copying and overlay mounting in a single loop — for each partition, it copies files then immediately mounts. If a copy fails on partition #3, partitions #1 and #2 are already overlay-mounted with no rollback path.

**Reference behavior:**
- **Meta-hybrid** (best): Copies to `.tmp_<module_id>`, atomic `rename()` to final name, `.backup_<module_id>` for rollback. Parallel via rayon `par_iter`. If anything fails during staging, no mounts have happened yet.
- **Mountify**: Bulk `cp -Lrf` all module content into staging, then mounts. No atomic rename, but staging is separated from mounting.

**Decision:** Split `execute_overlay()` into two phases following meta-hybrid's pattern:

**Phase 1 — Stage all lower dirs:**
For each partition mount, for each contributing module:
1. `prepare_lower_dir()` copies files into a `.tmp_<mod_id>` directory (existing function, unchanged)
2. `fs::rename()` atomically moves `.tmp_<mod_id>` to the final lower dir name
3. If any staging fails, no overlay mounts have been created — `cleanup_storage()` removes the staging area cleanly

**Phase 2 — Mount all overlays:**
Only runs if all staging succeeded. For each partition mount, call `mount_overlay()` with the staged lower dirs.

**Why atomic rename matters:** `fs::rename()` on the same filesystem is a single inode operation in the kernel — it either completes or doesn't. No half-written directory state. This is the critical reliability improvement over the current interleaved approach.

**Why NOT full `.backup_` rollback from meta-hybrid:** We already call `cleanup_storage()` on any error, which removes the entire staging directory. The backup/restore pattern is for incremental updates where you want to preserve previous state — we rebuild from scratch each boot, so the backup adds complexity with no benefit.

**Files to change:** `src/mount/executor.rs` (split the `for pm in &plan.partition_mounts` loop into two passes, add `.tmp_` staging dir and `fs::rename`)

**What stays unchanged:** `prepare_lower_dir()` function (executor.rs:108-149), `mount_overlay()` call signature, `StorageHandle` struct, `cleanup_storage()`. Only the orchestration loop in `execute_overlay()` changes.

---

### D2-5: Mount propagation control (MS_PRIVATE)

**Severity:** MEDIUM (reliability)
**What it is:** After creating the storage backend (tmpfs/erofs/ext4), we mount overlays on top. Without setting mount propagation to PRIVATE, these overlay mounts can leak into child mount namespaces — meaning apps that create their own namespace (e.g., via `unshare`) could see or be affected by our mounts in unpredictable ways.

**Reference behavior:**
- **Meta-hybrid**: Sets PRIVATE propagation on EROFS backend, tmpfs workspace, and magic mount directories (3 locations)
- **Mountify**: Not documented / not found

**Decision:** ADD. After `init_storage()` succeeds in `execute_overlay()`, call `libc::mount` with `MS_PRIVATE` on the storage base path. One syscall, prevents mount event propagation to child namespaces.

**Implementation detail:**
```rust
// After init_storage() in execute_overlay():
unsafe {
    libc::mount(
        std::ptr::null(),
        storage.base_path.as_ptr(),
        std::ptr::null(),
        libc::MS_PRIVATE,
        std::ptr::null(),
    );
}
```

**Files to change:** `src/mount/executor.rs` (add one `libc::mount` call after `init_storage()` in both `execute_overlay` and `execute_magic_mount`)

---

### D2-6: SELinux context on ext4 image file

**Severity:** MEDIUM (reliability)
**What it is:** When ZeroMount creates an ext4 loop image for overlay storage, the image file itself has no SELinux context set. The kernel may generate SELinux denials when accessing the loop device backing, because the file inherits whatever context its parent directory has (likely `u:object_r:adb_data_file:s0`) instead of an appropriate context.

**Current state:** We already mirror SELinux contexts on individual files inside the overlay (executor.rs:134,143 calls `mirror_selinux_context`). The gap is the ext4 image FILE that backs the loop mount.

**Reference behavior:**
- **Meta-hybrid**: Sets `u:object_r:ksu_file:s0` on the image file, then `u:object_r:system_file:s0` on all content inside
- **Mountify**: Sets `u:object_r:ksu_file:s0` on ext4 images during KSU mode

**Decision:** ADD. After creating the ext4 image file in `try_ext4_storage()`, set SELinux context `u:object_r:ksu_file:s0` on the image file via `lsetxattr`. Both references use this exact context for KSU.

**Files to change:** `src/mount/storage.rs` (add `lsetxattr` call after ext4 image creation in `try_ext4_storage()`), may need `src/utils/selinux.rs` if the helper doesn't support setting arbitrary contexts (currently only mirrors)

---

### D2-8: Module sort order — reverse alphabetical

**Severity:** LOW (alignment)
**What it is:** When multiple modules contribute files to the same partition overlay, the order they appear in the lower dir list determines which module's file "wins" on conflicts. Currently, our order depends on iteration order of the `HashMap` in `execute_overlay()` (executor.rs:39-40) — effectively random per run.

**Reference behavior:**
- **Meta-hybrid**: Reverse alphabetical by module ID (`scanner.rs:109` in their codebase)
- **Mountify**: Unsorted directory scan (filesystem order)

**Decision:** ALIGN with meta-hybrid — sort modules reverse alphabetically by ID. Deterministic ordering prevents non-reproducible overlay stacking behavior across reboots. Reverse alphabetical means a module named `zzz-fonts` would be processed before `aaa-tweaks`, which means `zzz-fonts` files appear in lower layers (lower priority in overlay) and `aaa-tweaks` files appear in upper layers (higher priority in overlay).

**Files to change:** `src/modules/scanner.rs` (add `.sort_by(|a, b| b.id.cmp(&a.id))` after module scanning, or in `execute_plan` before passing to executor)

---

### D1-5: Remove scanner binary call from metainstall.sh

**Severity:** MEDIUM (reliability)
**What it is:** When KSU installs another module, it calls our `metainstall.sh` hook. Line 21 runs `zeromount module scan --update-conf`, which invokes the full Rust binary at install time. This adds latency to the install flow and risks hanging the KSU install UI if the binary blocks.

**Reference behavior:**
- **Mountify**: metainstall.sh does NOT run any binary. Only exports env vars, overrides handle_partition, calls install_module, and creates symlinks.
- **Meta-hybrid**: metainstall.sh does NOT run any binary. Only exports env vars, overrides handle_partition, calls install_module, and creates symlinks.

**Additional evidence:** In a previous session, agents discovered that `--update-conf` appears to be effectively a no-op — `handlers.rs:193-194` shows the flag only logs a debug message (`"partitions.conf rebuild requested"`) without persisting anything. The scan output goes to stdout but isn't used.

**Decision:** REMOVE line 21 from `metainstall.sh`. Module changes are picked up at next boot when the pipeline runs via service.sh. No binary invocation during install.

**Files to change:** `module/metainstall.sh` (remove line 21: `"$MODDIR/bin/${ABI}/zeromount" module scan --update-conf 2>/dev/null`)

---

### D1-7: Shell-level single-instance lock

**Severity:** MEDIUM (reliability)
**What it is:** Currently, instance locking only exists in the Rust binary (`lock.rs` uses flock with LOCK_NB). There is no shell-level protection against double execution of shell scripts. Mountify documented that KSU fires `post-fs-data` twice in metamodule mode, which is why they added a lock file.

**Reference behavior:**
- **Mountify**: Creates `/dev/mountify_single_instance` lock file at start of post-fs-data.sh, removes at end of service.sh. Simple file-exists check, not flock.
- **Meta-hybrid**: No shell-level lock documented.

**Current state after RC3:** metamount.sh no longer invokes the Rust binary — it just calls `ksud kernel notify-module-mounted` and exits. The double-execution risk is reduced but not eliminated: if KSU fires metamount.sh twice, we'd call notify-module-mounted twice (harmless), and if service.sh somehow fires twice, the Rust-level try-lock handles it.

**Decision:** ADD a simple lock file at `/dev/zeromount_lock`. Create at the start of service.sh (the only script that invokes the binary after RC3). Check existence before proceeding. Remove on exit. Matches mountify's pattern. Defense in depth — the Rust try-lock is the primary guard, this is belt-and-suspenders.

**Files to change:** `module/service.sh` (add lock file creation/check at top, trap-based cleanup at exit)

---

### D1-9: service.sh should exit 1 on failure

**Severity:** LOW (alignment)
**What it is:** `service.sh` currently exits 0 on unsupported architecture (line 9: `exit 0`). This masks failures — if the binary can't run, nothing reports the error.

**Reference behavior:**
- **Mountify**: Not documented
- **Meta-hybrid**: No service.sh (uses metamount.sh only)

**Decision:** Change `exit 0` to `exit 1` for the unsupported-arch fallback in service.sh. Also ensure the binary's exit code propagates — if zeromount mount fails, service.sh should exit non-zero. KSU doesn't act on service.sh exit codes, but correct exit behavior aids debugging via `logcat` and `dmesg`.

**Files to change:** `module/service.sh` (change `exit 0` to `exit 1` on line 9, capture binary exit code)

---

### D8-4: /proc/kallsyms full read — NO ACTION

**Severity:** MEDIUM (timing)
**What it is:** `storage.rs:693` reads the entire `/proc/kallsyms` file via `fs::read_to_string` to find the `ext4_unregister_sysfs` symbol address for the LKM nuke. This is blocking I/O that takes ~50-200ms depending on kernel symbol count.

**Reference behavior:**
- **Mountify**: Same pattern — reads `/proc/kallsyms` for the same symbol, with kptr_restrict manipulation
- **Meta-hybrid**: Does NOT read /proc/kallsyms (doesn't use ext4 LKM nuke)

**Decision:** NO ACTION. Our approach matches mountify exactly. The read is bounded (kallsyms is a finite kernel-generated file), completes in <200ms, and only runs when ext4 mode with LKM nuke is active. The surrounding `nuke_ext4_sysfs` subprocess calls already get timeouts from RC5.

**Files to change:** None

---

## Decision Summary

### All decisions at a glance

| ID | Decision | Action | Priority |
|---|---|---|---|
| **RC1** | Kill watcher from boot path | Remove `start_module_watcher()` from handlers.rs | P0 — deadlock fix |
| **RC2** | Fix bootloop protector (4 bugs) | Rewrite guard in pipeline.rs, remove reset in config.rs, add shell guard to service.sh | P0 — deadlock fix |
| **RC3** | Single pipeline at service.sh only | metamount.sh becomes shell-only notify, remove post-fs-data pipeline path | P0 — deadlock fix |
| **RC4** | Leave try-lock as-is | None | — |
| **RC5** | Timeout all subprocess calls | Extract run_command_with_timeout to utils, apply to platform.rs and storage.rs | P1 — reliability |
| **V1-V5** | Resolved by RC1-RC3 | None (covered above) | — |
| **V6** | Rename module ID to meta-zeromount | Ripple through module.prop, constants.ts, config paths, scripts | P2 — spec compliance |
| **V7** | No action (handle_partition no-op is correct) | None | — |
| **V8** | Dropped (not a violation) | None | — |
| **V9** | Declare manage.kernel_umount false | Add to customize.sh | P2 — spec compliance |
| **D2-4** | Two-phase overlay staging with atomic rename | Split execute_overlay() into stage + mount passes | P1 — reliability |
| **D2-5** | Add MS_PRIVATE mount propagation | One libc::mount call after init_storage() | P1 — reliability |
| **D2-6** | SELinux context on ext4 image file | lsetxattr ksu_file:s0 on image after creation | P1 — reliability |
| **D2-8** | Reverse alphabetical module sort | Sort in scanner or executor | P2 — alignment |
| **D1-5** | Remove scanner from metainstall.sh | Delete line 21 | P1 — reliability |
| **D1-7** | Shell-level lock file in service.sh | Lock file at /dev/zeromount_lock | P2 — defense in depth |
| **D1-9** | Exit 1 on failure in service.sh | Change exit codes | P2 — alignment |
| **D8-4** | No action (matches mountify) | None | — |
| **F1** | Fix listPackages() to use official API | Replace 3 fake methods with `ksu.listPackages(type)` | P3 — WebUI compliance |
| **F2** | Rewrite ksu.d.ts to match kernelsu@3.0.0 | Add 8 official methods, remove 4 undocumented, fix types | P3 — WebUI compliance |
| **F3** | Implement spawn() for streaming | Add ChildProcess wrapper per official pattern | P3 — WebUI compliance |
| **F4** | Fix PackagesInfo type (isSystemApp → isSystem) | Covered by F2 | P3 — WebUI compliance |
| **F5** | Replace getPackagesIcons() with ksu://icon/ URL | Remove function, use URL scheme in components | P3 — WebUI compliance |
| **F6** | Keep custom wrapper, remove duplicate | Remove api.ts:29-58 duplicate, single wrapper in ksuApi.ts | P3 — WebUI compliance |
| **F7** | Add enableInsets(true) call | One-line addition on app init | P3 — WebUI compliance |
| **F8** | Use moduleInfo() instead of hardcoded paths | Replace constants.ts hardcoded paths (ties to V6 rename) | P3 — WebUI compliance |
| **F9** | Add MODDIR to uninstall.sh | One-line fix | P3 — scripts compliance |
| **F10** | Extract ABI detection to common.sh | New common.sh sourced by 6 scripts | P3 — scripts compliance |
| **W13** | Exclude webroot from set_perm_recursive | Narrow scope in customize.sh | P3 — WebUI compliance |
| **W14** | Remove duplicate exec wrapper | Covered by F6 | P3 — WebUI compliance |
| **W15** | Validate UID before shell interpolation | Numeric check at api.ts:388 | P3 — WebUI compliance |
| **W16** | No action (Svelte auto-escapes) | Verify only | — |

### Implementation order

**Phase 1 (deadlock killers):** RC1, RC3, RC2 — in this order because RC1 is one line, RC3 simplifies metamount.sh, RC2 is the most complex.

**Phase 2 (reliability):** RC5, D2-4, D2-5, D2-6, D1-5 — subprocess timeouts first (shared utility), then executor refactor, then mount hardening, then metainstall cleanup.

**Phase 3 (alignment + compliance):** V6, V9, D2-8, D1-7, D1-9, F1-F10, W13-W15 — spec compliance, WebUI API alignment, and polish. F2 (ksu.d.ts rewrite) should go first in the WebUI batch since F1/F3/F4/F5 depend on it. F8 ties to V6 (module rename). F10 (common.sh) should go before other shell script changes.

---

## WebUI Compliance Findings (from KSU-COMPLIANCE-AUDIT.md)

**Status:** DECISIONS PENDING — queued for next session collaborative review.
**Reference doc for all WebUI decisions:** `/home/claudetest/zero-mount/reference/kernelsu-module-webui.md` (1087 lines)
**WebUI reference from mountify:** `/home/claudetest/zero-mount/context-gathering/output/mountify/parts/webui.md` (572 lines)
**WebUI reference from meta-hybrid:** `/home/claudetest/zero-mount/context-gathering/output/meta-hybrid_mount/parts/webui.md` (640 lines)

### FAIL Findings (8) — need decisions

**F1. listPackages() uses undocumented native method names** (HIGH)
- File: `webui/src/lib/ksuApi.ts:70`
- Issue: Calls `ksu.listAllPackages()`, `ksu.listUserPackages()`, `ksu.listSystemPackages()` — none exist in KSU API
- Expected: `ksu.listPackages(type)` where type is "user", "system", or "all" (kernelsu-module-webui.md Section 4, Section 7)
- Impact: Primary code path fails silently, falls back to slow `pm list packages` shell command
- Found by: webui-auditor, webui-xcheck (independent agreement)
- **Decision:** FIX. Replace the three undocumented method calls with `ksu.listPackages(type)`. The function at line 67 already accepts `type: 'all' | 'user' | 'system'` — just pass it through to the single native method instead of mapping to three non-existent methods. Keep the `pm list packages` shell fallback for non-KSU environments.
- **Files to change:** `webui/src/lib/ksuApi.ts` (rewrite lines 67-90: replace methodMap dispatch with single `ksu.listPackages(type)` call), `webui/src/lib/ksu.d.ts` (replace `listAllPackages/listUserPackages/listSystemPackages` with `listPackages(type: string): string`)

**F2. ksu.d.ts type definitions diverge from official API** (HIGH)
- File: `webui/src/lib/ksu.d.ts:16-24`
- Issue: Missing 6 documented APIs (spawn, fullScreen, enableInsets, toast, moduleInfo, listPackages), includes 4 undocumented ones
- Expected: Match kernelsu 3.0.0 TypeScript definitions (kernelsu-module-webui.md Section 5, Section 7)
- Found by: webui-auditor, webui-xcheck (independent agreement)
- **Decision:** FIX. Rewrite `ksu.d.ts` to match the official `kernelsu@3.0.0` TypeScript definitions. Add all 8 documented methods: `exec`, `spawn`, `fullScreen`, `enableInsets`, `toast`, `moduleInfo`, `listPackages`, `getPackagesInfo`. Remove 4 undocumented methods: `listAllPackages`, `listUserPackages`, `listSystemPackages`, `getPackagesIcons`. Fix `KsuPackageInfo`: rename `isSystemApp?` to `isSystem` (non-optional boolean), remove `targetSdkVersion` (not in spec), make `versionName`/`versionCode`/`uid` non-optional to match spec.
- **Files to change:** `webui/src/lib/ksu.d.ts` (full rewrite to match kernelsu-module-webui.md Section 5)

**F3. spawn() not implemented — no streaming for long operations** (MEDIUM)
- File: absent from all `webui/src/lib/` files
- Issue: Module scanning and loading use sequential calls with no progress feedback
- Expected: kernelsu-module-webui.md Section 4 recommends `spawn()` for long-running operations
- Found by: webui-auditor, webui-xcheck (independent agreement)
- **Decision:** FIX. Implement `spawn()` wrapper using the official `ksu.spawn(command, argsJson, optionsJson, callbackName)` native method. The ChildProcess pattern from kernelsu-module-webui.md Section 6 (lines 481-517) provides streaming stdout/stderr via event emitters. Use for module scanning and any operation that takes >1s so the UI can show progress. F2 decision already adds `spawn` to `ksu.d.ts`.
- **Files to change:** `webui/src/lib/ksuApi.ts` (add `spawn()` function following the ChildProcess pattern from kernelsu@3.0.0), callers in WebUI that currently use `ksuExec()` for long operations

**F4. getPackagesInfo type field name mismatch** (MEDIUM)
- File: `webui/src/lib/ksu.d.ts:8`
- Issue: `isSystemApp?: boolean` vs official `isSystem: boolean`; `targetSdkVersion` not in docs
- Expected: Match PackagesInfo Object table (kernelsu-module-webui.md Section 4)
- Found by: webui-auditor, webui-xcheck (independent agreement)
- **Decision:** FIX. Covered by F2 — `ksu.d.ts` rewrite changes `isSystemApp?: boolean` to `isSystem: boolean` and removes `targetSdkVersion`. Any code reading `isSystemApp` must be updated to read `isSystem` instead.
- **Files to change:** `webui/src/lib/ksu.d.ts` (covered by F2), any component referencing `isSystemApp` field

**F5. getPackagesIcons() uses undocumented API** (MEDIUM)
- File: `webui/src/lib/ksuApi.ts:136-161`
- Issue: Calls `ksu.getPackagesIcons()` — not in official API. Doc recommends `ksu://icon/{packageName}` URL scheme
- Found by: webui-auditor, webui-xcheck (independent agreement)
- **Decision:** FIX. Replace `ksu.getPackagesIcons()` native call with `ksu://icon/{packageName}` URL scheme per the docs. Set `img.src = "ksu://icon/" + packageName` directly — the KSU manager intercepts this URL scheme and returns the app icon. Remove the `getPackagesIcons` function from `ksuApi.ts` and `getPackagesIcons` from `ksu.d.ts` (already covered by F2). Callers should use the URL scheme directly in `<img>` tags or CSS background-image.
- **Files to change:** `webui/src/lib/ksuApi.ts` (remove `getPackagesIcons` function at lines 136-161), components that display app icons (switch to `ksu://icon/{packageName}` URL)

**F6. Does not use kernelsu npm package — reimplements bridge** (LOW)
- Files: `webui/src/lib/api.ts:29-58`, `webui/src/lib/ksuApi.ts:37-65`
- Issue: Two custom wrappers instead of importing from `kernelsu` npm package
- Assessment: Functionally correct (callback pattern matches doc Section 6), adds timeout improvement, but bypasses official typed API
- Found by: webui-auditor, webui-xcheck (independent agreement)
- **Decision:** KEEP our custom wrapper. It adds timeout handling that the official `kernelsu` npm package lacks. F1/F2 decisions already align our method signatures with the official API. The wrapper stays in `ksuApi.ts` but calls the correct native methods. Remove the duplicate in `api.ts` (W14) — single wrapper, correct signatures.
- **Files to change:** `webui/src/lib/api.ts` (remove duplicate exec wrapper at lines 29-58, import from ksuApi instead)

**F7. enableInsets() not used** (LOW)
- File: `webui/src/lib/App.tsx:53` — uses CSS `env(safe-area-inset-bottom)` fallback
- Issue: KSU provides `enableInsets()` for proper WebView inset handling (kernelsu-module-webui.md Section 4)
- Found by: webui-auditor, webui-xcheck (independent agreement)
- **Decision:** FIX. Add `ksu.enableInsets(true)` call on app initialization. The CSS `env(safe-area-inset-bottom)` fallback can stay as defense-in-depth for non-KSU environments. One line addition.
- **Files to change:** `webui/src/App.tsx` or app entry point (add `ksu?.enableInsets?.(true)` on mount)

**F8. moduleInfo() not used — paths hardcoded** (LOW)
- File: `webui/src/lib/constants.ts:2-8`
- Issue: All paths hardcode `/data/adb/modules/zeromount/`. Doc Section 13: "don't hardcode paths"
- Mitigated: `customize.sh:40-42` creates `bin/zm` at a stable path, so the hardcoding is at least consistent
- Note: V6 decision (rename to meta-zeromount) will force updating these paths anyway — consider using `moduleInfo()` at that time
- Found by: webui-auditor, webui-xcheck (independent agreement)
- **Decision:** FIX. Replace hardcoded paths in `constants.ts` with `ksu.moduleInfo()` call to get the module ID dynamically, then derive paths from it. V6 (rename to meta-zeromount) forces touching these paths anyway — use `moduleInfo()` instead of re-hardcoding the new name. Fallback to hardcoded path if `moduleInfo()` is unavailable (non-KSU environments).
- **Files to change:** `webui/src/lib/constants.ts` (replace hardcoded `/data/adb/modules/zeromount/` with `moduleInfo()`-derived paths)

### WARN Findings (4) — need decisions

**W13. webroot permissions set manually vs KSU doc warning** (MEDIUM)
- File: `module/customize.sh:60,65`
- Issue: Sets permissions and SELinux context on `$MODPATH` including `webroot/`, but KSU WebUI docs say "do not set the permissions for this directory yourself"
- Found by: structure-auditor + webui-auditor (cross-domain finding)
- **Decision:** FIX. Exclude `webroot/` from `set_perm_recursive` scope in `customize.sh`. KSU auto-handles webroot permissions and SELinux context. Our manual `set_perm_recursive` on `$MODPATH` applies to everything including webroot — either narrow the scope to `$MODPATH/bin` and other non-webroot dirs, or run `set_perm_recursive` then let KSU override webroot.
- **Files to change:** `module/customize.sh` (narrow `set_perm_recursive` at lines 60,65 to exclude `$MODPATH/webroot`)

**W14. Duplicate exec wrappers** (LOW)
- Files: `webui/src/lib/api.ts:29` + `webui/src/lib/ksuApi.ts:37`
- Issue: Two independent `execCommand` wrappers with different timeout behavior
- Assessment: Related to F6 — consolidating to `kernelsu` npm package would eliminate both
- Found by: webui-auditor, webui-xcheck (independent agreement)
- **Decision:** FIX. Covered by F6 decision — remove the duplicate exec wrapper in `api.ts:29-58`, have all callers import from `ksuApi.ts` instead. Single wrapper, single source of truth.
- **Files to change:** Covered by F6

**W15. Inconsistent shell escaping in includeUid()** (LOW)
- File: `webui/src/lib/api.ts:388`
- Issue: UID value passed to shell command without escaping — potential injection if UID is user-controlled
- Found by: webui-auditor
- **Decision:** FIX. Validate UID is a numeric integer before passing to shell command. UIDs are always integers — reject anything that isn't `^\d+$`. Prevents shell injection if the value is ever derived from untrusted input.
- **Files to change:** `webui/src/lib/api.ts` (add numeric validation before shell interpolation at line 388)

**W16. Shell output rendered in UI** (LOW)
- File: `webui/src/lib/api.ts:61-81`
- Issue: Raw shell command output displayed in UI without sanitization
- Assessment: Mitigated by Svelte's default HTML escaping, but defense-in-depth says sanitize anyway
- Found by: webui-auditor
- **Decision:** NO ACTION. Svelte's `{expression}` syntax auto-escapes HTML by default. Only `{@html}` bypasses escaping. As long as shell output is rendered via normal Svelte bindings (not `{@html}`), this is safe. Verify during implementation that no `{@html}` is used with shell output.
- **Files to change:** None (verify only)

### Scripts FAIL findings from compliance audit (also pending)

**F9. uninstall.sh missing MODDIR** (LOW)
- File: `module/uninstall.sh` (no MODDIR line)
- Issue: Doc says "In all scripts of your module, please use MODDIR"
- Impact: Script only uses absolute paths — functionally harmless
- **Decision:** FIX. Add `MODDIR="${0%/*}"` as line 2 of `uninstall.sh`. One-line fix for doc compliance. All other scripts already have this.
- **Files to change:** `module/uninstall.sh` (add MODDIR line)

**F10. Architecture detection duplicated 6 times** (LOW)
- Files: `customize.sh:13`, `post-fs-data.sh:6`, `service.sh:4`, `metainstall.sh:16`, `metamount.sh:4`, `metauninstall.sh:5`
- Issue: Identical `case` block in 6 scripts
- Assessment: Maintenance risk. Could extract to `common.sh` sourced by all scripts.
- **Decision:** FIX. Extract the `case "$(uname -m)"` ABI detection block to `module/common.sh`. All 6 scripts source it via `. "$MODDIR/common.sh"`. Single source of truth — one place to update if architecture mappings change. Matches KSU convention where `common.sh` is a recognized pattern.
- **Files to change:** `module/common.sh` (new — contains ABI detection case block), `module/customize.sh`, `module/post-fs-data.sh`, `module/service.sh`, `module/metainstall.sh`, `module/metamount.sh`, `module/metauninstall.sh` (replace inline case block with `. "$MODDIR/common.sh"`)
