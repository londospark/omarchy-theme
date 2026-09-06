//! Push-tier test: a swap performed into a temp state dir must surface as
//! exactly one callback (debounced, swap-burst tolerant).

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::mpsc;
use std::time::Duration;

use omarchy_theme::{Theme, ThemeWatcher};

fn theme_text(_name: &str, accent: &str) -> String {
    format!("mode = \"dark\"\nbackground = #111111\nforeground = #eeeeee\naccent = {accent}\nselection = #292e42\n")
}

fn write_direct(dir: &Path, name: &str, accent: &str) {
    fs::create_dir_all(dir.join("theme")).unwrap();
    fs::write(dir.join("theme.name"), format!("{name}\n")).unwrap();
    fs::write(dir.join("theme/colors.toml"), theme_text(name, accent)).unwrap();
}

/// Render into a staging dir and atomically swap it in, like theme-set does.
fn swap_in(dir: &Path, name: &str, accent: &str) {
    let staging = dir.join("staging");
    let _ = fs::remove_dir_all(&staging);
    fs::create_dir_all(staging.join("theme")).unwrap();
    fs::write(staging.join("theme/colors.toml"), theme_text(name, accent)).unwrap();
    fs::write(staging.join("theme.name"), format!("{name}\n")).unwrap();

    let graveyard = dir.join("graveyard");
    let _ = fs::remove_dir_all(&graveyard);
    let old = dir.join("theme");
    let new = staging.join("theme");
    fs::rename(&old, &graveyard).unwrap();
    fs::rename(&new, &old).unwrap();
    fs::rename(dir.join("staging/theme.name"), dir.join("theme.name")).unwrap();
    let _ = fs::remove_dir_all(&graveyard);
}

fn temp_dir(tag: &str) -> PathBuf {
    std::env::temp_dir().join(format!("omarchy-{tag}-{}", std::process::id()))
}

#[test]
fn watcher_delivers_one_callback_per_swap() {
    let dir = temp_dir("watch");
    let _ = fs::remove_dir_all(&dir);
    write_direct(&dir, "one", "#ff0000");

    let (tx, rx) = mpsc::channel::<String>();
    let watcher = ThemeWatcher::spawn_state_dir(&dir, move |t| {
        tx.send(t.palette.color("accent").unwrap().to_hex()).is_ok()
    })
    .unwrap();

    // initial delivery, then quiet
    assert_eq!(rx.recv_timeout(Duration::from_secs(3)).unwrap(), "#ff0000");
    assert!(
        rx.recv_timeout(Duration::from_millis(300)).is_err(),
        "no spurious events while idle"
    );

    swap_in(&dir, "two", "#0000ff");
    assert_eq!(rx.recv_timeout(Duration::from_secs(3)).unwrap(), "#0000ff");
    assert!(
        rx.recv_timeout(Duration::from_millis(400)).is_err(),
        "the swap's event burst must be coalesced to one callback"
    );

    drop(watcher);
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn theme_roundtrip_via_swap() {
    let dir = temp_dir("swap-round");
    let _ = fs::remove_dir_all(&dir);
    write_direct(&dir, "solo", "#123456");
    let t = Theme::from_state_dir(&dir).unwrap();
    assert_eq!(t.name.as_deref(), Some("solo"));
    assert_eq!(t.palette.color("accent").unwrap().to_hex(), "#123456");
    swap_in(&dir, "next", "#654321");
    assert!(t.changed());
    let t2 = Theme::from_state_dir(&dir).unwrap();
    assert_eq!(t2.name.as_deref(), Some("next"));
    assert!(!t2.changed());
    let _ = fs::remove_dir_all(&dir);
}
