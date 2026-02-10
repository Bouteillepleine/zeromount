# Known Issues — ZeroMount v2.0.0-dev

Observed on Redmi 14C (QCFAGMU8S8GAPNW8), Android 14, KernelSU Next, SUSFS v2.0.0.

---

## 7. Bad font module causes Zygote crash loop (appears as boot deadlock)

**Behavior:** Device gets stuck on MIUI logo after installing a font-replacement module (e.g., Facebook15.0 with 226 font files in `system/fonts/`). ADB remains reachable. `sys.boot_completed` never becomes 1. `init.svc.zygote` shows "restarting" in a loop.

**Root cause:** The module's replacement fonts are incompatible with MIUI's Typeface loader. SystemServer crashes on every boot attempt:
```
FATAL EXCEPTION IN SYSTEM PROCESS: main
NullPointerException: Attempt to read from field 'int android.graphics.Typeface.mStyle'
  on a null object reference
    at Typeface.create(Typeface.java:1020)
    at Typeface.setSystemFontMap(Typeface.java:1575)
    at Typeface.loadPreinstalledSystemFontMap(Typeface.java:1712)
    at SystemServer.run(SystemServer.java:1001)
```

**Why the bootloop protector doesn't help:** The ZeroMount pipeline completes successfully (226 VFS rules applied, scenario=Full, degraded=false). The crash happens afterwards in SystemServer — a different process, outside ZeroMount's visibility. The bootcount mechanism only tracks pipeline failures, not downstream system crashes caused by module content.

**Recovery:** `adb shell touch /data/adb/modules/<module>/disable && adb reboot`

**Fix direction:** ZeroMount could detect Zygote crash loops by monitoring `init.svc.zygote` state after pipeline completion. If Zygote restarts N times within a window, disable the most recently installed module and clear VFS rules. This would require the watcher or a post-boot health check.

---

## 1. BRENE path hiding returns EINVAL (-22) on re-runs

**Affected paths:** `/data/adb`, `/data/adb/modules`, `/data/adb/ksu`, `/cache/recovery`, `/data/cache/recovery`, `/data/local/tmp`

**Behavior:** `add_sus_path` returns `-22` (EINVAL) on the second pipeline run within the same boot. First run (metamount.sh at post-fs-data) succeeds silently. Second run (service.sh post-boot) fails for these paths.

**Likely cause:** SUSFS kernel module rejects duplicate path registrations. The pipeline runs twice per boot (post-fs-data + post-boot), and BRENE tries to register the same paths both times.

**Impact:** Cosmetic — paths are already hidden from the first run.

**Fix direction:** Either skip BRENE on re-runs, or check for `-22` and treat it as "already registered" (not an error).

---

## 2. BRENE map hiding returns ENOENT (-2) for absent paths

**Affected entries:** `/data/adb/modules/shamiko/lib`, `/data/adb/ksu/bin/zygisk`, `/data/adb/ap/bin/zygisk`, `libzygisk`, `zygisk.so`

**Behavior:** `add_sus_map` returns `-2` (ENOENT) because these paths/patterns don't exist on this device.

**Impact:** None — these are best-effort hide attempts for common zygisk locations. shamiko isn't installed, APatch isn't present.

**Fix direction:** Pre-check existence before calling `add_sus_map`, or suppress `-2` from log output for map entries.

---

## 3. Per-file kstat spoofing fails with error -2 despite kstat_redirect: true

**Behavior:** Font files get `kstat_redirect failed ... error -2` from `add_sus_kstat_redirect`.

**Evidence (2025-03-14 boot):**
- `.detection.json` reports `kstat_redirect: true` (SUSFS v2.0.0 now supports it)
- Log: `kstat_redirect failed for /system/fonts/NotoSerifThai-Bold.ttf: kernel returned error -2 (open_redirect OK)`
- open_redirect succeeds but kstat_redirect fails for the same paths

**Impact:** Font metadata not spoofed. Detection tools comparing inode/timestamps could detect the VFS redirection.

**Fix direction:** The `-2` (ENOENT) likely means the target path doesn't exist yet at the time kstat_redirect is called. The VFS rule creates the redirect, but the original file's stat data may not be resolvable. Investigate call ordering.

---

## 4. Per-file path hiding fails for entire "clean" module (412 failures)

**Behavior:** Every file gets `path hiding failed ... error -2` from `add_sus_path`.

**Evidence:**
- Log: `path hiding failed module=clean path=/data/adb/modules/clean/system/bin/cat error=add_sus_path: kernel returned error -2` (repeated 412 times)

**Impact:** Module source files under `/data/adb/modules/clean/system/bin/` are visible in directory listings. The VFS overlay works (`succeeded=1`), but the backing files aren't hidden.

**Fix direction:** Same investigation as issue 3 — may be related to SUSFS path registration ordering or a version mismatch.

---

## 5. WebUI shows duplicate modules from filesystem corruption

**Behavior:** The Modules tab shows "Basic Cleaner" twice despite the Rust scanner correctly deduplicating to `count=1`.

**Root cause:** The WebUI's `scanKsuModules()` in `api.ts` uses a shell glob (`for dir in /data/adb/modules/*/`) that independently hits the duplicate directory entries. The Rust inode dedup doesn't cover this code path.

**Status:** Fixed — added path-based dedup in `api.ts` after JSON parsing.

**Permanent fix:** Clean up the duplicate directory entry on-device:
```
adb shell "ls -lai /data/adb/modules/ | grep clean"
# Remove one of the duplicate entries (both point to same inode)
```

---

## 6. First pipeline run produces partial rules, second produces full set

**Behavior:** metamount.sh pipeline at post-fs-data: `rules=4, modules=1`. service.sh pipeline at post-boot: `rules=226, modules=1`.

**Verified (2025-03-14 boot, zeromount.log):**
- 1741942073: `mount pipeline started` → `modules scanned count=1` → `rules=4` (metamount.sh)
- 1741942078: `initial pipeline finished` → `rules=226` (service.sh post-boot)

**Root cause:** At post-fs-data time, module files are not fully extracted yet (KSU processes modules_update/ concurrently). Only 4 of 226 font files are visible to the scanner. By late_start, extraction is complete.

**Impact:** The post-boot pipeline overwrites the partial result. Functionally harmless — the authoritative run is always the post-boot one. The 4-rule run at post-fs-data wastes a few ioctl calls but completes in <1s.

**Fix direction:** Consider skipping the metamount.sh pipeline entirely and letting service.sh handle everything. The post-fs-data run provides no benefit since module extraction isn't complete yet.

---

## 8. Bootloop protector never actually protects against bootloops

**Affected code:** `pipeline.rs:527-536`, `config.rs:337-348`

**Four distinct bugs:**

**8a. Pipeline still runs on bootloop detection.** `run_pipeline_with_bootloop_guard()` detects bootloop (count >= 3), restores backup config, then runs the full pipeline anyway. If the pipeline itself (or its downstream effects like issue 7) causes the loop, swapping config changes nothing.

**8b. `restore_backup()` resets bootcount to 0.** `config.rs` calls `reset_bootcount()` inside `restore_backup()`. This creates an infinite cycle: count reaches 3 → restore resets to 0 → pipeline runs → crash → reboot → count 1 → 2 → 3 → reset to 0 → repeat forever.

**8c. `service.sh` has no shell-level bootcount guard.** `metamount.sh:17-18` has `[ "$COUNT" -ge 3 ] && exit 1`, but `service.sh` (which also runs the pipeline at late_start) has no such guard. Even if the shell guard fired in metamount.sh, service.sh would run the pipeline unconditionally.

**8d. Watcher callbacks increment bootcount during runtime.** `handlers.rs:37-38` calls `run_pipeline_with_bootloop_guard` for hot-reload. Module installs during runtime increment the bootcount, conflating runtime re-scans with boot failures.

**Impact:** The bootloop protector is non-functional. A bad module (issue 7) or slow overlay/magic mount path cannot be recovered from automatically.

**Fix direction:**
1. When bootloop detected, skip pipeline entirely — return safe-mode RuntimeState
2. Remove `reset_bootcount()` from `restore_backup()` — only reset in `finalize()`
3. Add shell guard to `service.sh` matching `metamount.sh`
4. Watcher callbacks should use `run_full_pipeline` directly, not the bootloop-guarded version

---

## 9. metamount.sh IS called by KSU (contradicts OUTSTANDING-ISSUES.md)

**Previous claim (OUTSTANDING-ISSUES.md §3):** "KSU does not auto-call custom-named scripts like metamount.sh"

**Verified false.** The zeromount.log from 2025-03-14 shows `"mount pipeline started"` (the `handle_mount(false)` path, triggered by `zeromount mount` without `--post-boot`) running BEFORE `"initial pipeline finished"` (the `handle_mount(true)` path from service.sh) on every boot. This is the metamount.sh code path.

**Evidence:**
- 1741942073: `mount pipeline started` → `rules=4` (metamount.sh at post-fs-data)
- 1741942078: `initial pipeline finished` → `rules=226` (service.sh at late_start)

KSU logcat shows `"Found metamodule in modules directory: /data/adb/modules/zeromount"` at multiple boot stages but does not explicitly log `exec metamount.sh`. The execution is confirmed by the Rust binary's own log output.

**Impact:** OUTSTANDING-ISSUES.md §3 ("CRITICAL: metamount.sh Is Never Called") is invalid. The pipeline runs twice per boot as designed — once at post-fs-data (partial) and once at late_start (full).
