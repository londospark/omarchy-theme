//! Theme-level tests against synthetic state dirs (no dependency on the
//! desktop's current theme), plus a live-state smoke test.

use std::fs;
use std::time::{Duration, Instant};

use omarchy_theme::{Mode, Theme};

struct TempState {
    dir: std::path::PathBuf,
}

impl TempState {
    fn new(tag: &str) -> TempState {
        let dir =
            std::env::temp_dir().join(format!("omarchy-theme-test-{tag}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(dir.join("theme")).unwrap();
        TempState { dir }
    }

    fn write_theme(&self, colors: &str, shell: Option<&str>, name: &str) {
        fs::write(self.dir.join("theme.name"), format!("{name}\n")).unwrap();
        fs::write(self.dir.join("theme/colors.toml"), colors).unwrap();
        match shell {
            Some(s) => fs::write(self.dir.join("theme/shell.toml"), s).unwrap(),
            None => {
                let _ = fs::remove_file(self.dir.join("theme/shell.toml"));
            }
        }
    }
}

impl Drop for TempState {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.dir);
    }
}

#[test]
fn loads_and_reads_style() {
    let state = TempState::new("load");
    state.write_theme(
        "mode = \"dark\"\nbackground = #111111\nforeground = #eeeeee\naccent = #00ff00\n",
        Some("[controls]\nnormal-color = \"#eeeeee\"\nnormal-fill-alpha = 0.05\n"),
        "testtheme",
    );

    let theme = Theme::from_state_dir(&state.dir).unwrap();
    assert_eq!(theme.name.as_deref(), Some("testtheme"));
    assert_eq!(theme.palette.mode(), Mode::Dark);
    assert_eq!(theme.palette.color("accent").unwrap().g, 255);
    assert!(theme.style.from_shell);
    assert_eq!(theme.style.normal.fill_alpha, 0.05);
    // absent tokens fall back to the documented template defaults
    assert_eq!(theme.style.hover.fill_alpha, 0.08);
    assert_eq!(theme.style.font_base_size, 12.0);
}

#[test]
fn change_detection_survives_atomic_swap() {
    let state = TempState::new("swap");
    state.write_theme(
        "background = #111111\nforeground = #eee\naccent = #ff0000\n",
        None,
        "one",
    );

    let theme = Theme::from_state_dir(&state.dir).unwrap();
    assert!(!theme.changed());

    // simulate omarchy-theme-set: render next theme, then swap the directory
    let staging = state.dir.join("next");
    fs::create_dir_all(staging.join("theme")).unwrap();
    fs::write(staging.join("theme.name"), "two\n").unwrap();
    fs::write(
        staging.join("theme/colors.toml"),
        "background = #111111\nforeground = #eee\naccent = #0000ff\n",
    )
    .unwrap();

    // signature not yet affected
    assert!(!theme.changed());

    // atomic swap: old theme out, new theme in (mv to tmp first for
    // rename-into-place ordering like the real script)
    let graveyard = state.dir.join("graveyard");
    fs::rename(state.dir.join("theme"), &graveyard).unwrap();
    fs::rename(
        staging.join("theme.name"),
        state.dir.join("theme.name.next"),
    )
    .unwrap();
    fs::rename(staging.join("theme"), state.dir.join("theme")).unwrap();
    fs::rename(
        &state.dir.join("theme.name.next"),
        state.dir.join("theme.name"),
    )
    .unwrap();
    let _ = fs::remove_dir_all(&graveyard);

    assert!(theme.changed(), "swap must be observable");

    let mut reloaded = theme.clone();
    assert!(reloaded.reload().unwrap());
    assert_eq!(reloaded.name.as_deref(), Some("two"));
    assert_eq!(reloaded.palette.color("accent").unwrap().b, 255);
    assert!(!reloaded.changed());
}

#[test]
fn deleted_theme_momentarily_is_not_a_change() {
    // A swap burst has the dir briefly missing; changed() must not flap on a
    // stat failure that resolves itself (capture returns Err, treated as
    // "no change yet" so callers see exactly one change per swap).
    let state = TempState::new("flap");
    state.write_theme("background = #111111\nforeground = #eee\n", None, "flap");
    let theme = Theme::from_state_dir(&state.dir).unwrap();
    fs::remove_dir_all(state.dir.join("theme")).unwrap();
    assert!(!theme.changed());
}

#[test]
fn timing_stay_sane() {
    // the poll tier promises a sub-millisecond check; guard the API shape
    let state = TempState::new("timing");
    state.write_theme("background = #111111\nforeground = #eee\n", None, "t");
    let theme = Theme::from_state_dir(&state.dir).unwrap();
    let start = Instant::now();
    for _ in 0..10_000 {
        let _ = theme.changed();
    }
    let per = start.elapsed() / 10_000;
    assert!(per < Duration::from_micros(500), "changed() cost {per:?}");
}

#[test]
fn live_desktop_smoke() {
    // Running on a real Omarchy session this must succeed; elsewhere it
    // must fail cleanly rather than panic.
    match Theme::current() {
        Ok(theme) => {
            assert!(theme.palette.color("background").is_some());
            assert!(theme.palette.color("foreground").is_some());
        }
        Err(omarchy_theme::Error::NoTheme) | Err(omarchy_theme::Error::Io(_)) => {}
        Err(e) => panic!("unexpected live-theme error: {e}"),
    }
}
