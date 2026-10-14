// SPDX-License-Identifier: MPL-2.0

//! Privileged backend for cosmic-nightlight.
//!
//! This is the only part of the project that runs as root (via `pkexec`).
//! It keeps a deliberately tiny surface: parse a target temperature and
//! brightness, then ask `nightlight-core` to write the gamma LUTs to DRM,
//! performing the VT bounce required under COSMIC.
//!
//! Usage:
//!   cosmic-nightlight-helper --temp <kelvin> [--brightness <0.0-1.0>] [--session-vt <n>]
//!   cosmic-nightlight-helper --off          (reset to a neutral ramp)
//!   cosmic-nightlight-helper --version      (app version + CLI contract number)
//!
//! Those arguments are a frozen contract, because the flatpak build installs a
//! copy of this binary on the host separately from ordinary app updates.
//! `--version` also reports a minimum security revision so security fixes trigger
//! authenticated host setup independently of CLI changes. A newer GUI checks
//! that the older host copy still understands
//! it; see `nightlight_core::HELPER_CONTRACT` before changing any of them.
//!
//! `--session-vt` is the caller's graphical-session VT (from `XDG_VTNR`, which
//! `pkexec` strips from the environment). When given, the VT bounce only runs
//! if that VT is foreground and always switches back to it; without it the
//! helper falls back to snapshotting the foreground VT (manual CLI use).

use std::process::ExitCode;

struct Args {
    kelvin: u32,
    brightness: f64,
    session_vt: Option<i32>,
}

fn parse_args(args: impl Iterator<Item = String>) -> Result<Args, String> {
    let mut kelvin: Option<u32> = None;
    let mut brightness: f64 = 1.0;
    let mut session_vt: Option<i32> = None;
    let mut off = false;

    let mut args = args;
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--off" => off = true,
            "--temp" | "-t" => {
                let v = args.next().ok_or("--temp requires a value")?;
                kelvin = Some(v.parse().map_err(|_| format!("invalid temperature: {v}"))?);
            }
            "--brightness" | "-b" => {
                let v = args.next().ok_or("--brightness requires a value")?;
                brightness = v.parse().map_err(|_| format!("invalid brightness: {v}"))?;
            }
            "--session-vt" => {
                let v = args.next().ok_or("--session-vt requires a value")?;
                session_vt = Some(v.parse().map_err(|_| format!("invalid session VT: {v}"))?);
            }
            "--help" | "-h" => {
                return Err("help".to_string());
            }
            other => return Err(format!("unknown argument: {other}")),
        }
    }

    let kelvin = if off {
        nightlight_core::NEUTRAL_KELVIN
    } else {
        kelvin.ok_or("missing required --temp <kelvin> (or use --off)")?
    };

    if !(0.0..=1.0).contains(&brightness) {
        return Err(format!(
            "brightness must be between 0.0 and 1.0 (got {brightness})"
        ));
    }

    if session_vt.is_some_and(|vt| !(1..=63).contains(&vt)) {
        return Err("session VT must be between 1 and 63".to_owned());
    }

    Ok(Args {
        kelvin,
        brightness,
        session_vt,
    })
}

fn usage() {
    eprintln!(
        "usage: cosmic-nightlight-helper --temp <kelvin> [--brightness <0.0-1.0>] [--session-vt <n>]\n       cosmic-nightlight-helper --off\n       cosmic-nightlight-helper --version"
    );
}

fn main() -> ExitCode {
    // Answered before anything else, and deliberately before the root check: the
    // GUI asks the *installed* helper which command line it speaks, and that
    // question must not cost a password prompt. See `nightlight_core::
    // HELPER_CONTRACT` for why the answer is a contract number, not a version.
    if std::env::args()
        .skip(1)
        .any(|arg| arg == "--version" || arg == "-V")
    {
        println!("{}", nightlight_core::version_line());
        return ExitCode::SUCCESS;
    }

    let args = match parse_args(std::env::args().skip(1)) {
        Ok(args) => args,
        Err(msg) => {
            if msg != "help" {
                eprintln!("cosmic-nightlight-helper: {msg}");
            }
            usage();
            return ExitCode::FAILURE;
        }
    };

    if !nightlight_core::is_root() {
        eprintln!(
            "cosmic-nightlight-helper: must run as root (it switches VTs and grabs DRM master).\n\
             Invoke it via pkexec, not directly."
        );
        return ExitCode::FAILURE;
    }

    match nightlight_core::apply(args.kelvin, args.brightness, args.session_vt) {
        Ok(applied) => {
            println!(
                "cosmic-nightlight-helper: applied {}K @ {:.0}% to {} CRTC(s)",
                args.kelvin,
                args.brightness * 100.0,
                applied.crtcs
            );
            if applied.crtcs == 0 {
                // Treat "nothing updated" as a failure so a caller that retries
                // (the daemon) tries again — at login the displays may not be
                // up yet. Exit non-zero rather than reporting a false success.
                eprintln!(
                    "cosmic-nightlight-helper: no CRTCs were updated (no active displays with gamma support yet?)"
                );
                return ExitCode::FAILURE;
            }
            ExitCode::SUCCESS
        }
        Err(err) => {
            eprintln!("cosmic-nightlight-helper: failed to apply gamma: {err}");
            ExitCode::FAILURE
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn accepts(args: &[&str]) -> bool {
        parse_args(args.iter().map(|arg| (*arg).to_owned())).is_ok()
    }

    #[test]
    fn rejects_nonfinite_brightness_and_invalid_vts() {
        for value in ["NaN", "inf", "-inf", "-0.1", "1.1"] {
            assert!(!accepts(&["--temp", "3500", "--brightness", value]));
        }
        for value in ["0", "-1", "64", "2147483647", "x"] {
            assert!(!accepts(&["--off", "--session-vt", value]));
        }
        assert!(accepts(&[
            "--temp",
            "3500",
            "--brightness",
            "0.8",
            "--session-vt",
            "2"
        ]));
        assert!(accepts(&["--off"]));
    }

    #[test]
    fn rejects_unknown_and_incomplete_arguments() {
        for args in [
            vec![],
            vec!["--temp"],
            vec!["--off", "--session-vt"],
            vec!["--off", ";sh"],
        ] {
            assert!(!accepts(&args));
        }
    }
}
