/*
 * OmaVeil - An Omarchy-native window minimizer for Hyprland
 *
 * Forked from NiflVeil (https://github.com/somtooo/NiflVeil)
 * Original work Copyright (C) 2024 Maui The Magnificent (Charon)
 * Original contact: Maui_The_Magnificent@proton.me
 *
 * Thanks to Charon for the original concept and clean implementation.
 * This fork replaces the EWW widget picker with Walker's dmenu mode,
 * making it a natural fit for Omarchy setups where Walker is already present.
*/

use std::{
    collections::HashMap,
    env,
    fs::{self, OpenOptions},
    io::{self, Write},
    path::Path,
    process::{Command, Stdio},
};

const CACHE_DIR: &str = "/tmp/minimize-state";
const CACHE_FILE: &str = "/tmp/minimize-state/windows.json";
const PREVIEW_DIR: &str = "/tmp/window-previews";
const LOG_FILE: &str = "/tmp/omaveil.log";
const ICONS: [(&str, &str); 10] = [
    ("firefox", ""),
    ("alacritty", ""),
    ("discord", "󰙯"),
    ("steam", ""),
    ("chromium", ""),
    ("code", "󰨞"),
    ("spotify", ""),
    ("ghostty", ""),
    ("kitty", ""),
    ("default", "󰖲"),
];

// Append a timestamped error line to /tmp/omaveil.log
fn log_error(msg: &str) {
    let timestamp = Command::new("date")
        .arg("+%Y-%m-%d %H:%M:%S")
        .output()
        .ok()
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .unwrap_or_else(|| "?".to_string());
    let timestamp = timestamp.trim();
    if let Ok(mut file) = OpenOptions::new().create(true).append(true).open(LOG_FILE) {
        let _ = writeln!(file, "[{}] ERROR: {}", timestamp, msg);
    }
}

#[derive(Clone)]
struct MinimizedWindow {
    address: String,
    display_title: String,
    class: String,
    original_title: String,
    preview_path: Option<String>,
    icon: String,
}

fn get_app_icon(class_name: &str) -> String {
    let lower = class_name.to_lowercase();
    ICONS
        .iter()
        .find(|(name, _)| lower.contains(*name))
        .map(|(_, icon)| *icon)
        .unwrap_or(ICONS.last().unwrap().1)
        .to_string()
}

fn capture_window_preview(window_id: &str, geometry: &str) -> io::Result<String> {
    let preview_path = format!("{}/{}.png", PREVIEW_DIR, window_id);
    let thumb_path = format!("{}/{}.thumb.png", PREVIEW_DIR, window_id);

    Command::new("grim")
        .args(["-g", geometry, &preview_path])
        .output()?;

    Command::new("convert")
        .args([
            &preview_path,
            "-resize",
            "200x150^",
            "-gravity",
            "center",
            "-extent",
            "200x150",
            &thumb_path,
        ])
        .output()?;

    fs::remove_file(&preview_path)?;

    Ok(thumb_path)
}

fn create_json_output(windows: &[MinimizedWindow]) -> String {
    let mut output = String::from("[");
    for (i, window) in windows.iter().enumerate() {
        if i > 0 {
            output.push(',');
        }
        output.push_str(&format!(
            "{{\"address\":\"{}\",\"display_title\":\"{}\",\"class\":\"{}\",\"original_title\":\"{}\",\"preview\":\"{}\",\"icon\":\"{}\"}}",
            window.address,
            window.display_title.replace('"', "\\\""),
            window.class,
            window.original_title.replace('"', "\\\""),
            window.preview_path.as_ref().unwrap_or(&String::new()),
            window.icon
        ));
    }
    output.push(']');
    output
}

fn parse_window_info(info: &str) -> io::Result<HashMap<String, String>> {
    let mut result = HashMap::new();
    let content = info.trim_matches(|c| c == '{' || c == '}');

    for pair in content.split(',') {
        if let Some((key, value)) = pair.split_once(':') {
            let clean_key = key.trim().trim_matches('"');
            let clean_value = value.trim().trim_matches('"');
            result.insert(clean_key.to_string(), clean_value.to_string());
        }
    }

    Ok(result)
}

fn parse_windows_from_json(content: &str) -> io::Result<Vec<MinimizedWindow>> {
    let mut windows = Vec::new();
    let content = content.trim();

    if content.starts_with('[') && content.ends_with(']') {
        let content = &content[1..content.len() - 1];
        for window_json in content.split("},").map(|s| s.trim_end_matches(']')) {
            let window_json = window_json.trim_start_matches(',').trim();
            if window_json.is_empty() {
                continue;
            }

            let mut window_data = parse_window_info(window_json)?;
            windows.push(MinimizedWindow {
                address: window_data.remove("address").unwrap_or_default(),
                display_title: window_data.remove("display_title").unwrap_or_default(),
                class: window_data.remove("class").unwrap_or_default(),
                original_title: window_data.remove("original_title").unwrap_or_default(),
                icon: window_data.remove("icon").unwrap_or_default(),
                preview_path: Some(window_data.remove("preview").unwrap_or_default()),
            });
        }
    }

    Ok(windows)
}

fn restore_specific_window(window_id: &str) -> io::Result<()> {
    let output = Command::new("hyprctl")
        .args(["activeworkspace", "-j"])
        .output()?;

    if !output.status.success() {
        log_error(&format!(
            "restore: hyprctl activeworkspace failed for address={} — {}",
            window_id,
            String::from_utf8_lossy(&output.stderr).trim()
        ));
        return Ok(());
    }

    let workspace_info =
        String::from_utf8(output.stdout).expect("cannot get workspace_info from stdout");
    let workspace_data = parse_window_info(&workspace_info)?;
    let current_ws = workspace_data
        .get("id")
        .and_then(|id| id.parse().ok())
        .unwrap_or(1);

    let move_cmd = format!("{},address:{}", current_ws, window_id);
    let move_result = Command::new("hyprctl")
        .args(["dispatch", "movetoworkspace", &move_cmd])
        .output()?;

    if !move_result.status.success() {
        log_error(&format!(
            "restore: movetoworkspace failed for address={} — stdout={} stderr={}",
            window_id,
            String::from_utf8_lossy(&move_result.stdout).trim(),
            String::from_utf8_lossy(&move_result.stderr).trim()
        ));
    }

    let focus_result = Command::new("hyprctl")
        .args(["dispatch", "focuswindow", &format!("address:{}", window_id)])
        .output()?;

    if !focus_result.status.success() {
        log_error(&format!(
            "restore: focuswindow failed for address={} — stdout={} stderr={}",
            window_id,
            String::from_utf8_lossy(&focus_result.stdout).trim(),
            String::from_utf8_lossy(&focus_result.stderr).trim()
        ));
    }

    let content = fs::read_to_string(CACHE_FILE)?;
    let windows = parse_windows_from_json(&content)?;
    let updated_windows: Vec<MinimizedWindow> = windows
        .into_iter()
        .filter(|w| w.address != window_id)
        .collect();
    fs::write(CACHE_FILE, create_json_output(&updated_windows))?;

    Ok(())
}

fn restore_all_windows() -> io::Result<()> {
    let content = fs::read_to_string(CACHE_FILE)?;
    let windows = parse_windows_from_json(&content)?;

    for window in windows {
        restore_specific_window(&window.address)?;
    }

    Ok(())
}

/// Opens a Walker dmenu picker listing all minimized windows.
/// Uses index mode (-i) so we get back the 0-based position of the selection,
/// avoiding any text-mangling issues (e.g. walker stripping leading icon chars).
fn show_restore_menu() -> io::Result<()> {
    if !Path::new(CACHE_FILE).exists() {
        return Ok(());
    }

    let content = fs::read_to_string(CACHE_FILE)?;
    let windows = parse_windows_from_json(&content)?;

    if windows.is_empty() {
        return Ok(());
    }

    let input = windows
        .iter()
        .map(|w| format!("{} - {}", w.class, w.original_title))
        .collect::<Vec<_>>()
        .join("\n");

    let mut child = Command::new("walker")
        .args(["-d", "-i", "-p", "Restore window:"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .map_err(|e| {
            log_error(&format!("restore: failed to spawn walker — {}", e));
            e
        })?;

    if let Some(mut stdin) = child.stdin.take() {
        stdin.write_all(input.as_bytes()).map_err(|e| {
            log_error(&format!("restore: failed to write to walker stdin — {}", e));
            e
        })?;
    }

    let output = child.wait_with_output().map_err(|e| {
        log_error(&format!("restore: walker wait_with_output failed — {}", e));
        e
    })?;

    let raw = String::from_utf8_lossy(&output.stdout).trim().to_string();

    if raw.is_empty() {
        return Ok(());
    }

    match raw.parse::<usize>() {
        Ok(idx) if idx < windows.len() => {
            restore_specific_window(&windows[idx].address)?;
        }
        Ok(idx) => {
            log_error(&format!(
                "restore: walker returned index {} but only {} windows are minimized",
                idx,
                windows.len()
            ));
        }
        Err(e) => {
            log_error(&format!(
                "restore: could not parse walker output {:?} as index — {}",
                raw, e
            ));
        }
    }

    Ok(())
}

fn restore_window(window_id: Option<&str>) -> Result<(), io::Error> {
    match window_id {
        Some(id) => restore_specific_window(id),
        None => show_restore_menu(),
    }
}

fn minimize_window() -> Result<(), io::Error> {
    let output = Command::new("hyprctl")
        .args(["activewindow", "-j"])
        .output()?;

    if !output.status.success() {
        log_error(&format!(
            "minimize: hyprctl activewindow failed — {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ));
        return Ok(());
    }

    let window_info =
        String::from_utf8(output.stdout).expect("can't get window_info from output, stdout");
    let window_data = parse_window_info(&window_info)?;

    // Don't minimize walker itself (it's the picker UI)
    if window_data
        .get("class")
        .map_or(false, |c| c.to_lowercase() == "walker")
    {
        return Ok(());
    }

    let window_addr = window_data
        .get("address")
        .ok_or("No address found")
        .expect("error");
    let short_addr: String = window_addr.chars().rev().take(4).collect();
    let class_name = window_data
        .get("class")
        .ok_or("No class found")
        .expect("error");
    let title = window_data
        .get("title")
        .ok_or("No title found")
        .expect("error");
    let icon = get_app_icon(class_name);

    let geometry = window_data.get("at").and_then(|at| {
        window_data
            .get("size")
            .map(|size| format!("{},{}", at.trim(), size.trim()))
    });

    let preview_path = if let Some(geom) = geometry {
        capture_window_preview(window_addr, &geom).ok()
    } else {
        None
    };

    let window = MinimizedWindow {
        address: window_addr.to_string(),
        display_title: format!("{} {} - {} [{}]", icon, class_name, title, short_addr),
        class: class_name.to_string(),
        original_title: title.to_string(),
        preview_path,
        icon,
    };

    let dispatch_arg = format!("special:minimum,address:{}", window_addr);
    let output = Command::new("hyprctl")
        .args(["dispatch", "movetoworkspacesilent", &dispatch_arg])
        .output()?;

    if output.status.success() {
        let content = fs::read_to_string(CACHE_FILE)?;
        let mut windows = parse_windows_from_json(&content)?;
        windows.push(window);
        fs::write(CACHE_FILE, create_json_output(&windows))?;
    } else {
        log_error(&format!(
            "minimize: movetoworkspacesilent failed for class={} address={} — stdout={} stderr={}",
            class_name,
            window_addr,
            String::from_utf8_lossy(&output.stdout).trim(),
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }

    Ok(())
}

fn show_status() -> io::Result<()> {
    if !Path::new(CACHE_FILE).exists() {
        println!("{{\"text\":\"󰘸\",\"class\":\"empty\",\"tooltip\":\"No minimized windows\"}}");
        return Ok(());
    }
    let content = fs::read_to_string(CACHE_FILE)?;
    let windows = parse_windows_from_json(&content)?;
    let count = windows.len();

    if count > 0 {
        println!(
            "{{\"text\":\"󰘸 {}\",\"class\":\"has-windows\",\"tooltip\":\"{} minimized windows\"}}",
            count, count
        );
    } else {
        println!("{{\"text\":\"󰘸\",\"class\":\"empty\",\"tooltip\":\"No minimized windows\"}}");
    }

    Ok(())
}

fn main() -> io::Result<()> {
    fs::create_dir_all(CACHE_DIR)?;
    fs::create_dir_all(PREVIEW_DIR)?;

    if !Path::new(CACHE_FILE).exists() {
        fs::write(CACHE_FILE, "[]")?;
    }

    let args: Vec<String> = env::args().collect();
    let command = args.get(1).map(|s| s.as_str()).unwrap_or("");

    match command {
        "minimize" => {
            minimize_window()?;
        }
        "restore" => {
            let window_id = args.get(2).map(|s| s.to_string());
            restore_window(window_id.as_deref())?;
        }
        "restore-all" => {
            restore_all_windows()?;
        }
        "restore-last" => {
            if let Ok(content) = fs::read_to_string(CACHE_FILE) {
                if let Ok(windows) = parse_windows_from_json(&content) {
                    if let Some(window) = windows.last() {
                        restore_window(Some(&window.address.clone()))?;
                    }
                }
            }
        }
        "show" => {
            show_status()?;
        }
        _ => {
            eprintln!("OmaVeil - Omarchy-native window minimizer for Hyprland");
            eprintln!();
            eprintln!("Usage: omaveil <command> [window_address]");
            eprintln!();
            eprintln!("Commands:");
            eprintln!("  minimize       Hide the focused window into special:minimum");
            eprintln!("  restore        Open Walker dmenu picker to restore a window");
            eprintln!("  restore [addr] Restore a specific window by address");
            eprintln!("  restore-last   Restore the most recently minimized window");
            eprintln!("  restore-all    Restore all minimized windows");
            eprintln!("  show           Print Waybar-compatible JSON status");
            eprintln!();
            eprintln!("Errors: {}", LOG_FILE);
        }
    }
    Ok(())
}
