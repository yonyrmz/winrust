//! Tests of the persistent settings.

#[test]
fn the_settings_live_in_a_json_file() {
    let path = winrust_ui::state::state_path().expect("the configuration directory exists");
    assert!(path.ends_with("settings.json"), "{path:?}");
    assert!(path.is_absolute(), "{path:?}");
}

#[test]
fn the_settings_round_trip() {
    let state = winrust_ui::state::PersistedState {
        volume: 0.5,
        balance: -0.25,
        equalizer: vec![1.0, 2.0],
        equalizer_preamp: 3.0,
        library_roots: vec![std::path::PathBuf::from("/music")],
        last_view: Some("Library".to_string()),
    };

    let json = serde_json::to_string(&state).unwrap();
    let parsed: winrust_ui::state::PersistedState = serde_json::from_str(&json).unwrap();

    assert_eq!(parsed.volume, 0.5);
    assert_eq!(parsed.balance, -0.25);
    assert_eq!(parsed.equalizer, vec![1.0, 2.0]);
    assert_eq!(parsed.equalizer_preamp, 3.0);
    assert_eq!(parsed.library_roots.len(), 1);
    assert_eq!(parsed.last_view.as_deref(), Some("Library"));
}
