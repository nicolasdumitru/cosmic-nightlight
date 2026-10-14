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

The first published revision failed GitHub's audit: two quick-xml vulnerabilities,
two unsoundness warnings (lru and memmap2), and four unmaintained-package warnings
(paste, proc-macro-error2, rustybuzz and ttf-parser).

The follow-up replaces those dependencies rather than excluding advisories:

- quick-xml 0.41.0, memmap2 0.9.11 and lru 0.18.5 replace the affected versions.
- i18n-embed-fl 0.10.1 uses proc-macro-error3.
- resvg/usvg 0.48.1 uses harfrust/skrifa instead of rustybuzz.
- cosmic-text uses fontdb 0.24; Adwaita title text uses crossfont/FreeType instead
  of ab_glyph/owned_ttf_parser. This removes the remaining ttf-parser consumers.
- The macOS-only metal dependency uses pastey instead of paste.

Several upstream callers still constrain the old series. Their source snapshots
and narrow patches are in [vendor/README.md](../vendor/README.md), with licenses
and registry source checksums. The reviewed libcosmic revision remains unchanged.
These snapshots create an ongoing maintenance obligation; prefer upstreaming the
caller changes and removing them when compatible upstream releases are available.

The local RustSec 0.33.0 report against database commit
`6aed84668c23d7f535077a9fe918bf1fa0f6a5bc` now has zero vulnerabilities and zero
informational warnings, with all notice/unmaintained/unsound categories enabled
and no ignored advisories. This local library-based check does not check yanked
packages. The unchanged `cargo audit --deny warnings` command in GitHub CI is the
end-to-end audit gate; verify the run for the exact commit being installed.

The Linux graph still includes iced_wgpu/cryoglyph through libcosmic's Wayland
feature. Selecting the software renderer does not exclude their dependencies.

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
cargo test --locked -p cosmic-nightlight -p iced_tiny_skia security_migration_tests
cargo audit --deny warnings
```

For actual hardware, save work first. Test one manual warm tint and reset, then
lock/unlock, suspend/resume and a schedule transition before enabling autostart.
On hybrid laptops, confirm the enabled eDP connector's driver. The helper still
visits all DRM cards; this branch does not assume a stable card number or change
GPU routing. There has been no hardware validation on the target AMD/NVIDIA laptop.

## Results in the review environment (2026-10-15)

- Rust 1.99.0: workspace `cargo check` passed after the dependency migrations.
- `cargo test --locked --workspace`: 81 tests passed (68 GUI/backend, 11 core,
  2 helper), with no display/VT operations run by the tests.
- Two additional SVG regression tests passed: correct scaled BGRA pixels across
  the renderer-version bridge, and rejection of invalid SVG/zero dimensions.
- Actual shipped polkit rule: 144 allow/deny cases passed using Node's VM.
- The RustSec library audit described above found zero advisories or warnings.
  cargo-audit's application framework cannot run locally because this environment
  lacks `/proc/self/exe`; the local check uses its rustsec library directly.
- CI now includes a full GUI/workspace test job and the SVG regression tests, in
  addition to helper/policy checks and the strict dependency audit.
- Existing compiler warnings in upstream snapshots and GUI float inference are
  separate from RustSec advisories. They are not suppressed by the audit.
- Nix evaluation/build, live polkit authorization, actual terminal switching,
  macOS compilation and release-binary/source equivalence were not validated here.
