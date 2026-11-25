use serde_json::{Value, json};
use std::fs;
use std::path::{Path, PathBuf};

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn env_value(name: &str) -> String {
    std::env::var(name).unwrap_or_default()
}

fn command_output(binary: &str, args: &[&str]) -> String {
    std::process::Command::new(binary)
        .args(args)
        .output()
        .map_or_default(|output| {
            let text = String::from_utf8_lossy(&output.stdout);
            text.lines().next().unwrap_or("").trim().to_owned()
        })
}

fn identify_session(session_type: &str, desktop: &str) -> (String, String) {
    let desktop = desktop.to_ascii_lowercase();
    if session_type == "wayland" && desktop.contains("kde") {
        (
            format!(
                "kde-plasma-{}",
                command_output("kwin_wayland", &["--version"]).replace(' ', "-")
            ),
            "KDE Plasma 6 (Wayland)".to_owned(),
        )
    } else if session_type == "wayland" && desktop.contains("gnome") {
        ("gnome-wayland".to_owned(), "GNOME (Wayland)".to_owned())
    } else if session_type == "wayland" && desktop.contains("sway") {
        ("sway".to_owned(), "Sway".to_owned())
    } else if session_type == "wayland" && desktop.contains("hyprland") {
        ("hyprland".to_owned(), "Hyprland".to_owned())
    } else if session_type == "x11" {
        ("x11".to_owned(), "X11".to_owned())
    } else {
        ("unknown".to_owned(), format!("{desktop} ({session_type})"))
    }
}

fn main() {
    let root = repo_root();
    let config: Value = serde_json::from_str(
        &fs::read_to_string(root.join("app/src-tauri/tauri.conf.json")).expect("read tauri config"),
    )
    .expect("parse tauri config");
    let hud = &config["app"]["windows"][1];
    let shortcut_source =
        fs::read_to_string(root.join("app/src-tauri/src/shortcut.rs")).expect("read shortcut.rs");
    let portal_registered = shortcut_source.contains("GlobalShortcuts")
        && shortcut_source.contains("Super+Shift+Space");
    let x11_fallback = shortcut_source.contains("XGrabKey");

    let session_type = env_value("XDG_SESSION_TYPE");
    let desktop = env_value("XDG_CURRENT_DESKTOP");
    let (key, label) = identify_session(&session_type, &desktop);
    let version = if key.starts_with("kde-plasma-") {
        command_output("kwin_wayland", &["--version"])
    } else {
        command_output(&desktop.to_ascii_lowercase(), &["--version"])
    };

    let report = json!({
        "generated_at": "2026-11-25",
        "session": {
            "key": key,
            "label": label,
            "type": session_type,
            "desktop": desktop,
            "wayland_display": env_value("WAYLAND_DISPLAY"),
            "display": env_value("DISPLAY"),
            "version": version,
        },
        "hud_config": {
            "width": hud["width"],
            "height": hud["height"],
            "transparent": hud["transparent"],
            "always_on_top": hud["alwaysOnTop"],
            "decorations": hud["decorations"],
        },
        "shortcut": {
            "portal_registered": portal_registered,
            "x11_native_fallback": x11_fallback,
            "binding": "Super+Shift+Space",
        },
        "runtime_checks": {
            "config_opaque": hud["transparent"] == false,
            "config_always_on_top": hud["alwaysOnTop"] == true,
            "pixel_screenshot_available": false,
            "note": "agent-desktop cannot capture displays on KDE Wayland; pixel-level opacity and keep-above layering were not measured here",
        },
    });

    let out_dir = root.join(format!("docs/evidence/compat/{key}"));
    fs::create_dir_all(&out_dir).expect("create compat evidence dir");
    let out_file = out_dir.join("probe.json");
    fs::write(
        &out_file,
        serde_json::to_string_pretty(&report).expect("format report"),
    )
    .expect("write compat probe");
    println!("compat probe written to {}", out_file.display());
}
