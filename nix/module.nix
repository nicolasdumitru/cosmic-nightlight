# SPDX-License-Identifier: MPL-2.0
{ config, lib, pkgs, ... }:
let
  cfg = config.services.cosmic-nightlight;
  helperPath = "${cfg.package.helper}/bin/cosmic-nightlight-helper";
in
{
  options.services.cosmic-nightlight = {
    enable = lib.mkEnableOption "the COSMIC night-light applet and privileged helper";
    package = lib.mkOption {
      type = lib.types.package;
      default = pkgs.callPackage ./package.nix { };
      description = "Package with a helper passthru attribute, built from the same source.";
    };
    users = lib.mkOption {
      type = lib.types.listOf lib.types.str;
      default = [ ];
      example = [ "alice" ];
      description = "Users allowed passwordless gamma changes while locally active. Empty means no passwordless access.";
    };
  };
  config = lib.mkIf cfg.enable {
    environment.systemPackages = [ cfg.package ];
    security.polkit.enable = true;
    # Authorize only this immutable helper, not a mutable profile symlink, an
    # arbitrary /nix/store prefix, a shell, or every member of wheel.
    security.polkit.extraConfig = ''
      polkit.addRule(function(action, subject) {
        var users = ${builtins.toJSON cfg.users};
        if (action.id == "org.freedesktop.policykit.exec" &&
            action.lookup("program") == ${builtins.toJSON helperPath} &&
            subject.local && subject.active &&
            users.indexOf(subject.user) !== -1) {
          return polkit.Result.YES;
        }
      });
    '';
    # Background scheduling is opt-in through the application's settings.
  };
}
