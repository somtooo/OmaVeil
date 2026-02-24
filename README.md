# OmaVeil

An [Omarchy](https://omarchy.org)-native window minimizer for Hyprland. Sends windows to a hidden special workspace and retrieves them on demand — mimicking the minimize behaviour you'd expect from a traditional desktop environment.

Uses [Walker](https://walkerlauncher.com) as the restore picker, the same launcher already built into Omarchy. No EWW, no extra dependencies.

> Forked from [NiflVeil](https://github.com/somtooo/NiflVeil) by [Maui The Magnificent (Charon)](mailto:Maui_The_Magnificent@proton.me).  
> Thanks to Charon for the original concept and clean zero-dependency Rust implementation.

---

## How it works

- **Minimize** — moves the focused window to `special:minimum` (a hidden Hyprland special workspace) and saves its metadata to `/tmp/minimize-state/windows.json`.
- **Restore** — opens a Walker dmenu picker listing all minimized windows. Select one to bring it back to the current workspace and focus it. This is the same pattern as the clipboard picker already in Omarchy (`cliphist list | walker --dmenu | ...`).
- **Restore last** — skips the picker and immediately restores the most recently minimized window.
- **Restore all** — brings every minimized window back at once.

---

## Dependencies

| Dependency | Notes |
|---|---|
| **Hyprland** | Required — `hyprctl` must be in `$PATH` |
| **Walker** | Required — already present in Omarchy |
| **Rust / Cargo** | Build-time only |

---

## Installation

### Download (recommended)

Grab the latest binary from GitHub Releases and place it in a system PATH directory Hyprland can see:

```
/usr/local/bin/omaveil
```

### Build from source

```bash
cd omaveil
cargo build --release
sudo cp target/release/omaveil /usr/local/bin/
```

> Tested only on Omarchy.

---

## Keybindings

Add these to `~/.config/hypr/bindings.conf`:

```
# OmaVeil - window minimizer
bindd = SUPER, H, Minimize window, exec, omaveil minimize
bindd = SUPER, I, Browse minimized windows, exec, omaveil restore
bindd = SUPER, U, Restore last minimized, exec, omaveil restore-last
bindd = SUPER SHIFT, U, Restore all minimized, exec, omaveil restore-all
```

| Binding | Action |
|---|---|
| `Super + H` | Hide (minimize) focused window |
| `Super + I` | Open Walker picker to restore a specific window |
| `Super + U` | Restore the most recently hidden window |
| `Super + Shift + U` | Restore all hidden windows |

---

## CLI reference

```
omaveil <command> [window_address]

Commands:
  minimize       Hide the focused window into special:minimum
  restore        Open Walker dmenu picker to restore a window
  restore [addr] Restore a specific window by address
  restore-last   Restore the most recently minimized window
  restore-all    Restore all minimized windows
  show           Print Waybar-compatible JSON status
```

### Optional: Waybar module

The `show` command outputs a Waybar-compatible JSON string. If you want a status indicator in your bar, add this to `~/.config/waybar/config.jsonc`:

```jsonc
"custom/omaveil": {
    "format": "{}",
    "exec": "omaveil show",
    "on-click": "omaveil restore",
    "return-type": "json",
    "interval": "once",
    "signal": 9
}
```

> Note: Omarchy's default Waybar config already uses signal 8 for the screen recording indicator. Use signal 9 (or higher) for OmaVeil to avoid conflicts.

---

## State

Window state is persisted at `/tmp/minimize-state/windows.json` for the lifetime of the session. It is cleared on reboot (lives in `/tmp`).

## Debugging

Only errors are logged (successful operations are silent). Error entries are timestamped and written to:

```
/tmp/omaveil.log
```

If a window isn't restoring, check the log immediately after the failed action:

```bash
cat /tmp/omaveil.log
```

The log includes the failing `hyprctl` command along with stdout/stderr to show exactly what Hyprland rejected and why. The log is also cleared on reboot.

---

## License

MIT — see [LICENSE](LICENSE).  
Original work Copyright © 2024 Maui The Magnificent (Charon).
