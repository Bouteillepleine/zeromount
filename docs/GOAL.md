# Goal

## One-Sentence Summary

A Rust-based KernelSU/APatch metamodule that mounts modules via VFS redirection when kernel patches are present, falling back to OverlayFS or magic mount when they're not.

## Success Criteria

- [ ] Rust binary replaces all shell orchestration (~2500 lines) — only thin launchers remain
- [ ] VFS mode: modules functional with zero `/proc/mounts` entries on patched kernels
- [ ] Overlay mode: modules functional via OverlayFS on unpatched kernels
- [ ] 4-scenario detection works correctly (FULL, SUSFS_FRONTEND, KERNEL_ONLY, NONE)
- [ ] SUSFS integration via build-time patching — no fork maintained
- [ ] Boot time equal or faster than current shell-based implementation
- [ ] WebUI displays active scenario and per-module mount strategy
- [ ] Passes Play Integrity / Momo detection on test device

## Explicitly Out of Scope

- NOT supporting Magisk (no metamodule concept)
- NOT building a new kernel patch — existing zeromount.c patches are reused
- NOT replacing SUSFS — we consume its API, not reimplement it
- NOT building a persistent background daemon — on-demand CLI + inotify watcher only
- NOT supporting riscv64 Android (ARM64 + ARM32 + x86_64 + x86 supported)

## Why This Matters

The previous ZeroMount was a metamodule that didn't actually mount things — it relied entirely on VFS redirection but still claimed `metamodule=1`. This caused LSPosed instability (hiding other systems' mounts) and couldn't work on kernels without custom patches. The rewrite makes ZeroMount a proper mount manager that gracefully degrades based on kernel capabilities, while preserving the mountless VFS approach as a superior option when available.
