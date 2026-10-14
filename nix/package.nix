# SPDX-License-Identifier: MPL-2.0
{
  lib,
  rustPlatform,
  stdenv,
  libcosmicAppHook,
  pkg-config,
  fontconfig,
  libdrm,
}:
let
  src = lib.cleanSource ../.;
  version = "0.5.1-security1";
  # Every git source is pinned by Cargo.lock. This keeps a local checkout usable
  # without inventing a cargoHash. For distribution in nixpkgs, replace builtin
  # git fetching with outputHashes (or a verified cargoHash).
  cargoLock = {
    lockFile = ../Cargo.lock;
    allowBuiltinFetchGit = true;
  };
  helper = rustPlatform.buildRustPackage {
    pname = "cosmic-nightlight-helper";
    inherit src version cargoLock;
    cargoBuildFlags = [ "-p" "nightlight-helper" ];
    cargoTestFlags = [ "-p" "nightlight-core" "-p" "nightlight-helper" ];
    # Keep the privileged executable unwrapped and free of GUI dependencies.
    installPhase = ''
      runHook preInstall
      install -Dm755 target/${stdenv.hostPlatform.rust.cargoShortTarget}/release/cosmic-nightlight-helper $out/bin/cosmic-nightlight-helper
      runHook postInstall
    '';
  };
in
rustPlatform.buildRustPackage {
  pname = "cosmic-nightlight";
  inherit src version cargoLock;
  nativeBuildInputs = [ libcosmicAppHook pkg-config ];
  buildInputs = [ fontconfig libdrm ];
  cargoBuildFlags = [ "-p" "cosmic-nightlight" ];
  cargoTestFlags = [ "-p" "cosmic-nightlight" ];
  COSMIC_NIGHTLIGHT_PACKAGED_HELPER = "${helper}/bin/cosmic-nightlight-helper";
  postInstall = ''
    install -Dm644 data/io.github.cosmic_nightlight.desktop $out/share/applications/io.github.cosmic_nightlight.desktop
    install -Dm644 data/io.github.cosmic_nightlight.settings.desktop $out/share/applications/io.github.cosmic_nightlight.settings.desktop
    install -Dm644 data/io.github.cosmic_nightlight.metainfo.xml $out/share/metainfo/io.github.cosmic_nightlight.metainfo.xml
    cp -r data/icons $out/share/
  '';
  passthru = { inherit helper; };
  meta = {
    description = "COSMIC night light with a separately packaged privileged helper";
    homepage = "https://github.com/cosmic-nightlight/cosmic-nightlight";
    license = lib.licenses.mpl20;
    platforms = lib.platforms.linux;
    mainProgram = "cosmic-nightlight";
  };
}
