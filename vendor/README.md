# Dependency migration patches

These are source snapshots of **callers** that constrained dependencies to
vulnerable or unmaintained release series. They are not renamed vulnerable
libraries or audit exclusions. Cargo.lock retains the real names and checksums
of the fixed/replacement registry packages. The root `[patch]` tables select
these snapshots. Keep `cargo audit --deny warnings` strict when updating them.

| Snapshot | Upstream source | Local changes |
| --- | --- | --- |
| zbus_xml 5.1.1 | crates.io zbus_xml 5.1.1 | quick-xml 0.39 → 0.41.0 minimum |
| xkbcommon 0.7.0 | crates.io xkbcommon 0.7.0 | memmap2 0.8 → 0.9.11 minimum |
| cryoglyph 0.1.0 | iced-rs/cryoglyph commit e429a025df36ab8145708acb309080ae3deec17a | lru 0.16 → 0.18.2 minimum |
| cosmic-text 0.19.0 | crates.io cosmic-text 0.19.0 | fontdb 0.23 → 0.24, whose parser no longer uses ttf-parser |
| metal 0.33.0 | crates.io metal 0.33.0 | paste → maintained pastey 0.2, retaining the local import name; macOS-only dependency |
| iced_tiny_skia 0.14.0 | pop-os/libcosmic commit dc1cf9f00cbe2902a52166492654bb9fee8a73d1, iced/tiny_skia | resvg 0.45 → 0.48.1; bridge resvg's tiny-skia 0.12 renderer into the existing 0.11 RGBA buffer; two rendering regression tests |
| iced_wgpu 0.14.0 | same libcosmic commit, iced/wgpu | resvg 0.45 → 0.48.1 |
| iced_winit 0.14.0 | same libcosmic commit, iced/winit | use winit's crossfont/FreeType Adwaita title renderer instead of ab_glyph/owned_ttf_parser; retain decorations, X11, Wayland and accessibility |
| build_helpers 0.14.0 | same libcosmic commit, iced/build_helpers | standalone manifest for the patched iced crates; source unchanged |

resvg 0.48 uses harfrust/skrifa instead of rustybuzz. fontdb 0.24 supplies its
replacement parser. i18n-embed-fl is updated directly to 0.10.1 in Cargo.lock;
that upstream release uses proc-macro-error3 instead of proc-macro-error2.

Registry snapshots come from the checksum-verified Cargo cache. Git sources are
identified above. Original license files are included; iced snapshots inherit
iced's MIT license. Source code is unchanged except for iced_tiny_skia/vector.rs.
Manifests are normalized to make workspace-inherited metadata and dependencies
explicit. Unused examples, benchmarks, fixtures and nested lockfiles are omitted
where possible; Cargo.toml.orig files, when present, are upstream originals.

The crossfont path adds FreeType/fontconfig build and runtime dependencies.
This branch targets Linux. The macOS-only metal patch has not been compiled on
macOS. Moving to upstream releases containing these migrations is preferable to
keeping these snapshots indefinitely. Review both upstream changes and the local
manifest/source deltas before refreshing them.
