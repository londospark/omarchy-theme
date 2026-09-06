//! Palette conformance: every installed theme, resolved by our Rust port,
//! must match `omarchy-theme-color --all` exactly (as recorded in
//! conformance/vectors/). Regenerate vectors after Omarchy updates:
//! `./conformance/generate.sh`.

use std::fs;
use std::path::PathBuf;

fn vectors_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../conformance/vectors")
}

#[derive(serde::Deserialize)]
struct Vector {
    theme: String,
    colors_file: String,
    light_mode_marker: bool,
    resolved: std::collections::BTreeMap<String, String>,
}

#[test]
fn every_vector_matches_the_rust_resolver() {
    let dir = vectors_dir();
    assert!(
        dir.is_dir(),
        "missing conformance vectors at {}",
        dir.display()
    );
    let mut count = 0;

    for entry in fs::read_dir(&dir).unwrap() {
        let path = entry.unwrap().path();
        if path.extension().map(|e| e != "json").unwrap_or(true) {
            continue;
        }
        let vector: Vector = serde_json::from_str(&fs::read_to_string(&path).unwrap())
            .unwrap_or_else(|e| panic!("bad vector {}: {e}", path.display()));
        count += 1;

        let raw = read_source_colors(&vector);
        let palette = omarchy_theme::Palette::resolve(&raw, vector.light_mode_marker);

        let mut problems = Vec::new();
        for (key, want) in &vector.resolved {
            match palette.get(key) {
                Some(got) if got == want => {}
                Some(got) => problems.push(format!("  {key}: got {got}, want {want}")),
                None => problems.push(format!("  {key}: missing, want {want}")),
            }
        }
        for (key, got) in palette.entries() {
            if !vector.resolved.contains_key(key) {
                problems.push(format!("  {key}: extra value {got}"));
            }
        }
        assert!(
            problems.is_empty(),
            "theme {} diverged from the oracle:\n{}",
            vector.theme,
            problems.join("\n")
        );
    }
    assert!(count >= 20, "only {count} vectors found");
}

/// Vectors are self-contained: generate.sh records the exact source bytes as
/// `<theme>.colors.toml` beside each JSON. The sidecar is authoritative (CI
/// machines have no Omarchy); the original path is consulted only for
/// vectors generated before sidecars existed.
fn read_source_colors(vector: &Vector) -> String {
    let sidecar = vectors_dir().join(format!("{}.colors.toml", vector.theme));
    if sidecar.is_file() {
        return fs::read_to_string(sidecar).unwrap();
    }
    let path = PathBuf::from(&vector.colors_file);
    if path.is_file() {
        return fs::read_to_string(path).unwrap();
    }
    panic!("colors source missing: {}", vector.colors_file);
}

#[test]
fn staged_theme_matches_source_resolution() {
    // The staged current theme has had overlays applied by omarchy itself;
    // resolving its colors.toml must still match its own oracle vector.
    let staged = vectors_dir().join("staged-coaster.json");
    if !staged.is_file() {
        return; // generation machine had no staged theme
    }
    let vector: Vector = serde_json::from_str(&fs::read_to_string(staged).unwrap()).unwrap();
    let raw = read_source_colors(&vector);
    let palette = omarchy_theme::Palette::resolve(&raw, vector.light_mode_marker);
    for (key, want) in &vector.resolved {
        assert_eq!(palette.get(key), Some(want.as_str()), "staged {key}");
    }
}
