# KernelSU Documentation Compliance Audit — Final Report

**Project:** metamodule-experiment (ZeroMount v2.0.0-dev)
**Audit Date:** 2026-02-10
**Method:** 10 independent agents across 5 domains, paired cross-validation
**Reference Docs:** kernelsu-module-guide.md, kernelsu-module-webui.md, kernelsu-module-config.md, kernelsu-additional-docs.md

---

## Executive Summary

| Domain | PASS | FAIL | WARN | Agent Agreement |
|--------|------|------|------|-----------------|
| Module Structure | 24 | 0 | 5 | Full (0 disputes) |
| Shell Scripts | 30 | 2 | 7 | High (1 revision) |
| WebUI | 11 | 8 | 4 | High (minor count diff) |
| Config/OverlayFS | 27 | 0 | 7 | High (1 false FAIL retracted) |
| Rust Backend | 39 | 0 | 5 | Full (0 disputes) |
| **TOTAL** | **131** | **10** | **28** | |

**Verdict:** The Rust backend and module structure are fully KSU-compliant. Shell scripts have minor gaps. The WebUI layer has the most divergence — primarily from not using the official kernelsu npm package APIs.

---

## FAIL Findings (10 total)

### HIGH Severity

**F1. WebUI: listPackages() uses undocumented native method names**
- File: ksuApi.ts:70
- Issue: Calls ksu.listAllPackages(), ksu.listUserPackages(), ksu.listSystemPackages() — none exist in KSU API
- Expected: ksu.listPackages(type) where type is "user", "system", or "all" (doc Section 4, Section 7)
- Impact: Primary code path fails silently, falls back to slow pm list packages shell command
- Found by: webui-auditor, webui-xcheck (independent agreement)

**F2. WebUI: ksu.d.ts type definitions diverge from official API**
- File: ksu.d.ts:16-24
- Issue: Missing 6 documented APIs (spawn, fullScreen, enableInsets, toast, moduleInfo, listPackages), includes 4 undocumented ones
- Expected: Match kernelsu 3.0.0 TypeScript definitions (doc Section 5, Section 7)
- Found by: webui-auditor, webui-xcheck (independent agreement)

### MEDIUM Severity

**F3. WebUI: spawn() not implemented — no streaming for long operations**
- File: (absent from all webui/src/lib/ files)
- Issue: Module scanning and loading use sequential calls with no progress feedback
- Expected: Doc Section 4 recommends spawn() for long-running operations
- Found by: webui-auditor, webui-xcheck (independent agreement)

**F4. WebUI: getPackagesInfo type field name mismatch**
- File: ksu.d.ts:8
- Issue: isSystemApp?: boolean vs official isSystem: boolean; targetSdkVersion not in docs
- Expected: Match PackagesInfo Object table (doc Section 4)
- Found by: webui-auditor, webui-xcheck (independent agreement)

**F5. WebUI: getPackagesIcons() uses undocumented API**
- File: ksuApi.ts:136-161
- Issue: Calls ksu.getPackagesIcons() — not in official API. Doc recommends ksu://icon/{packageName} URL scheme
- Found by: webui-auditor, webui-xcheck (independent agreement)

### LOW Severity

**F6. WebUI: Does not use kernelsu npm package — reimplements bridge**
- Files: api.ts:29-58, ksuApi.ts:37-65
- Issue: Two custom wrappers instead of importing from kernelsu
- Assessment: Functionally correct (callback pattern matches doc Section 6), adds timeout improvement, but bypasses official typed API
- Found by: webui-auditor, webui-xcheck (independent agreement)

**F7. WebUI: enableInsets() not used**
- File: App.tsx:53 — uses CSS env(safe-area-inset-bottom) fallback
- Issue: KSU provides enableInsets() for proper WebView inset handling (doc Section 4)
- Found by: webui-auditor, webui-xcheck (independent agreement)

**F8. WebUI: moduleInfo() not used — paths hardcoded**
- File: constants.ts:2-8
- Issue: All paths hardcode /data/adb/modules/zeromount/. Doc Section 13: "don't hardcode paths"
- Mitigated: customize.sh:40-42 creates bin/zm at a stable path, so the hardcoding is at least consistent
- Found by: webui-auditor, webui-xcheck (independent agreement)

**F9. Scripts: uninstall.sh missing MODDIR**
- File: uninstall.sh (no MODDIR line)
- Issue: Doc says "In all scripts of your module, please use MODDIR"
- Impact: Script only uses absolute paths — functionally harmless
- Found by: scripts-auditor (scripts-xcheck downgraded to WARN)

**F10. Scripts: Architecture detection duplicated 6 times**
- Files: customize.sh:13, post-fs-data.sh:6, service.sh:4, metainstall.sh:16, metamount.sh:4, metauninstall.sh:5
- Issue: Identical case block in 6 scripts
- Assessment: Maintenance risk, not a KSU doc violation per se
- Found by: scripts-auditor

---

## WARN Findings (28 total)

### Structure (5)
| # | Finding | File | Agents |
|---|---------|------|--------|
| W1 | Description contains emoji (ghost) | module.prop:6 | Both |
| W2 | updateJson= empty string | module.prop:7 | Both |
| W3 | Module ID zeromount doesn't follow meta- prefix recommendation | module.prop:1 | Both |
| W4 | aapt binary missing from x86/x86_64 dirs | module/bin/ | structure-xcheck |
| W5 | metauninstall.sh uses $1 instead of $MODULE_ID env var | metauninstall.sh:10 | structure-xcheck |

### Scripts (7)
| # | Finding | File | Agents |
|---|---------|------|--------|
| W6 | metainstall.sh overrides undocumented handle_partition() | metainstall.sh:11 | scripts-auditor |
| W7 | MODDIR unreliable when sourced | metainstall.sh:3 | Both |
| W8 | source=KSU requirement delegated to Rust binary | metamount.sh:20 | Both |
| W9 | Uses uname -m instead of documented $ARCH variable | customize.sh:13 | Both |
| W10 | No scripts check $KSU environment variable | All scripts | scripts-auditor |
| W11 | Custom env vars exported (KSU_HAS_METAMODULE, KSU_METAMODULE) | metainstall.sh:6-8 | Both |
| W12 | Bootloop guard exits with code 1 | metamount.sh:18 | scripts-xcheck |

### WebUI (4)
| # | Finding | File | Agents |
|---|---------|------|--------|
| W13 | webroot perms AND SELinux set manually vs doc warning | customize.sh:60,65 | Cross-domain |
| W14 | Duplicate wrappers | api.ts:29 + ksuApi.ts:37 | Both |
| W15 | Inconsistent shell escaping in includeUid() | api.ts:388 | webui-auditor |
| W16 | Shell output rendered in UI (mitigated by framework) | api.ts:61-81 | webui-auditor |

### Config/OverlayFS (7)
| # | Finding | File | Agents |
|---|---------|------|--------|
| W17 | skip_mount filter hides script-only modules from status | scanner.rs:55 | config-auditor |
| W18 | Magic mount doesn't mirror SELinux context | magic.rs (absent) | Both |
| W19 | Legacy overlay has no upperdir | overlay.rs:262-264 | config-xcheck |
| W20 | Overlay path escape doesn't handle colons | overlay.rs:150-152 | config-xcheck |
| W21 | Magic mount creates mount points on target filesystem | magic.rs:142-148 | config-xcheck |
| W22 | ABI detection duplicated across scripts | 4 scripts | config-xcheck |
| W23 | $MAGISK env var may not be formally defined | platform.rs:196 | Both |

### Rust Backend (5)
| # | Finding | File | Agents |
|---|---------|------|--------|
| W24 | module.prop parser doesn't parse updateJson, metamodule, etc. | scanner.rs:318-328 | Both |
| W25 | versionCode parsed as u32 (docs say integer) | types.rs:157 | rust-auditor |
| W26 | Magisk detection uses $MAGISK (not $MAGISK_VER_CODE) | platform.rs:196 | Both |
| W27 | No handling of update flag file | scanner.rs | rust-auditor |
| W28 | manage.kernel_umount not declared | (absent) | rust-xcheck |

---

## Cross-Domain Findings (discovered through agent collaboration)

1. **webroot permissions conflict** (structure-auditor + webui-auditor): customize.sh:60,65 sets permissions and SELinux context on $MODPATH including webroot/, but KSU WebUI docs say "do not set the permissions for this directory yourself."

2. **Binary path consistency** (scripts-auditor + webui-auditor): customize.sh:40-42 copies ABI binary to bin/zm, and constants.ts:2 hardcodes this path. Consistent.

3. **Boot timing verified** (scripts-auditor + config-xcheck + rust-auditor): Two-stage boot is correct — metamount.sh runs primary mount at post-fs-data, service.sh runs post-boot watcher. Config-auditor's false claim of "missing metamount.sh" was caught by cross-validation.

4. **source=KSU verified end-to-end** (scripts-auditor to rust-auditor): Shell delegates to binary, binary sets source via fsconfig_set_string(fs_fd, "source", "KSU") at overlay.rs:172. Propagation chain: storage.rs:352 to executor.rs:65 to overlay.rs:172.

5. **CLI surface map** (scripts-auditor):
   - zeromount detect (post-fs-data.sh)
   - zeromount mount (metamount.sh)
   - zeromount mount --post-boot (service.sh)
   - zeromount module scan --update-conf (metainstall.sh)
   - zeromount module scan --cleanup ID (metauninstall.sh)

---

## Audit Process Notes

- Cross-validation caught a false FAIL: config-auditor reported metamount.sh and metauninstall.sh as missing. Both files exist (verified by team-lead direct read, structure-auditor, structure-xcheck, scripts-auditor, scripts-xcheck, config-xcheck).

- Cross-domain messaging produced finding W13 (webroot permissions) that no single-domain audit would have caught.

- WebUI findings had highest agreement: Both auditors independently found the same core issues.

---

## Recommendations (Priority Order)

1. Fix listPackages() — Use ksu.listPackages(type) instead of undocumented methods (ksuApi.ts:70)
2. Update ksu.d.ts — Add all 8 documented native methods, remove 4 undocumented ones
3. Implement spawn() — For module scanning/loading progress feedback
4. Fix getPackagesInfo types — isSystemApp to isSystem, remove optionals
5. Use moduleInfo() — Replace hardcoded paths in constants.ts
6. Add MODDIR to uninstall.sh — One-line fix for doc compliance
7. Consider manage.kernel_umount — Declare to prevent KSU interference with overlay mounts
8. Remove webroot from set_perm_recursive scope — Or accept KSU auto-handling
9. Consolidate wrappers — Remove duplicate in ksuApi.ts, use single execCommand
10. Extract ABI detection — Shared function sourced by all 6 scripts
