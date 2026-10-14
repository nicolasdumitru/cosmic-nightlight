# Security hardening and residual risks

This branch is a hardening proposal, not a security certification. Its base is
`5b5a73d29a62fe05f0939ded6c3c0367038dc50d` (the Rust source and original lockfile
also match v0.5.1).

## Privileged boundary

The app stays unprivileged. Only the helper changes VTs and obtains DRM master.
The generic polkit rule now grants passwordless access only to active, local
members of wheel/sudo and only for the three exact installed helper paths. Other
requests fall through to the system policy; this is not a global deny rule and
does not override another administrator-installed authorization rule. Session
activity does not necessarily mean the screen is unlocked.

The helper serializes the entire switch/apply/restore operation using a root-owned
0600 regular file at `/run/cosmic-nightlight-helper.lock`. It rejects symlinks,
multiple hard links, unsafe permissions and overlapping invocations. It does not
truncate or unlink the file, avoiding inode replacement races between helpers.
Both packaged and manually invoked copies use the same lock. GUI runtime locks
remain useful for coalescing requests but are no longer the only coordination.

VT activation waits now time out after three seconds instead of blocking in
VT_WAITACTIVE. Restoration is attempted on failure and panic; an explicit restore
failure is reported to the caller. SIGKILL, process crashes, driver failures and
kernel stalls can still prevent restoration. A timeout is not a guarantee that
the display is recoverable without logging out or rebooting.

Brightness must be finite and within 0..1. Session VTs must be 1..63. The helper
still deliberately permits brightness zero and clamps color temperature. There
is no arbitrary command, output-path or plugin argument.

## Updating the host helper

`--version` now includes `(security 1)` separately from `(contract 1)`. Flatpak
requires the minimum security revision before treating a host install as ready.
A legacy helper triggers the existing authenticated setup path on the next
user-requested apply. Automatic reconciliation does not initiate a password
prompt. Future privileged security fixes must increment
`HELPER_SECURITY_REVISION`, even if the CLI remains compatible.

This is a minimum-revision check, not binary verification or an automatic update
service. Already-installed apps cannot enforce it until they themselves update.
Existing legacy helper processes do not participate in the new lock. Restart the
app and let any old invocation finish when updating. Native package managers must
update the app, helper and policy together. The NixOS module does this by referring
to the exact immutable helper store path from the same source build.

## Dependency status

The lockfile updates anyhow to 1.0.104, event-listener to 5.4.2, memmap2's 0.9
instance to 0.9.11, and wayland-scanner to 0.31.11 (using quick-xml 0.41.0).
They resolve RUSTSEC-2026-0190, RUSTSEC-2026-0221, the 0.9 instance of
RUSTSEC-2026-0186, and the Wayland scanner's XML issues.

The following matches remain. They have NOT been suppressed in CI:

| Dependency | Advisory | Reviewed use / remaining work |
| --- | --- | --- |
| quick-xml 0.39.4 | RUSTSEC-2026-0194, RUSTSEC-2026-0195 | zbus_xml 5.1.1 requires this series. The resolved path is through zbus-lockstep and its proc macros used by atspi-common. Upgrade that upstream chain; no claim of universal non-reachability is made. |
| memmap2 0.8.0 | RUSTSEC-2026-0186 | xkbcommon 0.7.0 requires it. Its inspected mapping call uses MmapOptions/map_copy_read_only, not the advisory's affected advise/flush range APIs. Updating iced's xkbcommon dependency remains preferable. |
| lru 0.16.4 | RUSTSEC-2026-0253 | cryoglyph pins the 0.16 series. Its glyph cache uses a Copy CacheKey (no panicking Drop), unlike the advisory's prerequisites. Upgrade the upstream dependency rather than relying indefinitely on this analysis. |

The Linux dependency graph DOES contain iced_wgpu/cryoglyph through libcosmic's
Wayland feature, even though the application does not explicitly select the
wgpu renderer. Renderer selection alone is not evidence that those crates are
absent. Maintenance warnings for paste, proc-macro-error2, rustybuzz and ttf-parser
also remain in the lockfile.

The security workflow runs `cargo audit --deny warnings`, without an ignore list.
It is expected to fail while these findings remain. This is intentionally visible;
a passing helper test suite does not mean the dependency audit is clean. Verify
reachability again after changes to features, lockfile or upstream libraries.

## Build supply chain

The source installer uses `cargo build --locked`. GitHub Actions and the Flatpak
Cargo generator are pinned to immutable commits. Compilation jobs have read-only repository permission; only separate publishing
jobs can write releases. Checkout does not persist its authentication token. Dependabot proposes Cargo and action updates weekly.
Pinning does not audit a dependency or establish reproducible release binaries;
the Flatpak SDK, Python build dependencies and Rust stable toolchain still move.

## Validation before daily use

Run the checks below without root; none should access graphics hardware:

```sh
cargo test --locked -p nightlight-core -p nightlight-helper
node tests/polkit.mjs
cargo test --locked -p cosmic-nightlight
cargo audit --deny warnings
```

For actual hardware, save work first. Test one manual warm tint and reset, then
lock/unlock, suspend/resume and a schedule transition before enabling autostart.
On hybrid laptops, confirm the enabled eDP connector's driver. The helper still
visits all DRM cards; this branch does not assume a stable card number or change
GPU routing. There has been no hardware validation on the target AMD/NVIDIA laptop.

## Results in the review environment (2026-10-15)

- Rust 1.99.0: `cargo check --locked -p cosmic-nightlight` passed.
- `cargo test --locked --workspace`: 81 tests passed (68 GUI/backend, 11 core,
  2 helper), with no display/VT operations run by the tests.
- Actual shipped polkit rule: 144 allow/deny cases passed using Node's VM.
- Shell syntax, workflow YAML parsing and `git diff --check` passed.
- Lockfile versions were checked against RustSec commit
  `6aed84668c23d7f535077a9fe918bf1fa0f6a5bc`, using Rust's semver crate to evaluate
  patched/unaffected requirements. Four security/unsoundness advisory matches and
  four unmaintained warnings remain, as described above. Yanked-package checks
  were not performed.
- cargo-audit 0.22.2 compiled here, but its application framework panicked because
  this environment lacks `/proc/self/exe`; no successful cargo-audit run is claimed.
  The unmodified command is configured in GitHub CI for a normal Linux runner.
- Existing float-inference warnings in the GUI and a proc-macro-error2 future
  compatibility warning remain. They are separate from this hardening work.
- Nix evaluation/build, live polkit authorization, actual terminal switching,
  and release-binary/source equivalence were not validated here.
