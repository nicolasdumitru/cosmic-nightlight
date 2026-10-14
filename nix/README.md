# NixOS integration (experimental)

These expressions target recent nixpkgs with `libcosmicAppHook`, including the
user's revision `7fd4004d0511991fff3a0524856a9be09e28c5f0`. They need an actual Nix
build and laptop validation before being treated as production-ready. No successful
Nix evaluation or build is claimed by the source-level review.

Keep a checkout of this branch and import `nix/module.nix` in your system config:

```nix
{
  imports = [ /absolute/path/to/cosmic-nightlight/nix/module.nix ];
  services.cosmic-nightlight = {
    enable = true;
    users = [ "your-local-login" ];
  };
}
```

Use your **local Linux username**, not necessarily your GitHub login. The list is
empty by default, granting nobody passwordless access. The policy additionally
requires an active local session and the exact helper store path. It does not
whitelist the entire Nix store, a shell, or a user-writable wrapper.

First run `nixos-rebuild build` to evaluate and build without activating. Review
`docs/security.md`, including the remaining dependency advisories, before switching.
After activation, add the Night Light applet through COSMIC's panel settings.
No user service is enabled automatically. Existing NixOS GPU configuration should
be left alone; the app does not need NVIDIA offload to tint an AMD-driven panel.

The GUI embeds `COSMIC_NIGHTLIGHT_PACKAGED_HELPER` at build time. Its helper is a
separate unwrapped derivation with no GUI runtime dependency closure. The policy
references that same immutable path. Changes to either package source update the
reference as part of the NixOS generation. Do not run `scripts/install.sh` or the
Flatpak host installer on top of this module; their conventional paths differ
from NixOS and they create state outside the declarative package installation.

The initial expression uses `cargoLock.allowBuiltinFetchGit` with exact Cargo.lock
revisions. A nixpkgs submission should instead record verified git `outputHashes`
or a verified `cargoHash`. No placeholder hash is presented as a verified one.

To remove the integration, disable the module and rebuild. Turn the tint off
before removal; uninstalling software does not necessarily reset a programmed
hardware gamma ramp. Log out/reboot if the compositor does not restore it.
