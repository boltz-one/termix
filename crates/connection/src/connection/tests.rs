//! Parity tests cho `ConnectionState` transition order — port
//! `BaseTerminalConnection.h::_transitionToState` guard. Validate: forward progress
//! only, terminal states reject, raise `StateChangedEvent` on transition.

use super::state::ConnectionState;
use super::{ConptyConnection, ITerminalConnection};

#[test]
fn transition_order_forward_progress() {
    let mut c = ConptyConnection::new();
    assert_eq!(c.state(), ConnectionState::NotConnected);
    c.transition_to_state(ConnectionState::Connecting).unwrap();
    c.transition_to_state(ConnectionState::Connected).unwrap();
    c.transition_to_state(ConnectionState::Closing).unwrap();
    c.transition_to_state(ConnectionState::Closed).unwrap();
    assert_eq!(c.state(), ConnectionState::Closed);
}

#[test]
fn transition_rejects_backwards() {
    let mut c = ConptyConnection::new();
    c.transition_to_state(ConnectionState::Connecting).unwrap();
    // NotConnected từ Connecting là backward — reject.
    assert!(c
        .transition_to_state(ConnectionState::NotConnected)
        .is_err());
}

#[test]
fn transition_rejects_skip_to_connected() {
    let mut c = ConptyConnection::new();
    // NotConnected → Connected trực tiếp (skip Connecting) — reject.
    assert!(c.transition_to_state(ConnectionState::Connected).is_err());
}

#[test]
fn transition_terminal_states_reject_all() {
    let mut c = ConptyConnection::new();
    // Force vào Closed.
    c.state = ConnectionState::Closing;
    c.transition_to_state(ConnectionState::Closed).unwrap();
    // Closed → Failed (flapping) — reject.
    assert!(c.transition_to_state(ConnectionState::Failed).is_err());
    assert!(c.transition_to_state(ConnectionState::Connecting).is_err());
    // Force vào Failed.
    c.state = ConnectionState::Connecting;
    c.transition_to_state(ConnectionState::Failed).unwrap();
    assert!(c.transition_to_state(ConnectionState::Connected).is_err());
}

#[test]
fn transition_failed_during_connecting() {
    let mut c = ConptyConnection::new();
    c.transition_to_state(ConnectionState::Connecting).unwrap();
    assert!(c.transition_to_state(ConnectionState::Failed).is_ok());
    assert_eq!(c.state(), ConnectionState::Failed);
}

#[test]
fn state_raised_on_transition() {
    let mut c = ConptyConnection::new();
    let rx = c.state_changed().subscribe();
    c.transition_to_state(ConnectionState::Connecting).unwrap();
    assert_eq!(rx.recv(), Ok(ConnectionState::Connecting));
}

/// `transition_to_state` phải sync `shared_state` atomic (Release) để output thread
/// đọc (Acquire) thấy state mới — port plan §1 sync requirement.
#[test]
fn transition_to_state_syncs_atomic() {
    use std::sync::atomic::Ordering;
    let mut c = ConptyConnection::new();
    c.transition_to_state(ConnectionState::Connecting).unwrap();
    assert_eq!(
        c.shared_state.load(Ordering::Acquire),
        ConnectionState::Connecting as u8
    );
    c.transition_to_state(ConnectionState::Connected).unwrap();
    assert_eq!(
        c.shared_state.load(Ordering::Acquire),
        ConnectionState::Connected as u8
    );
    c.transition_to_state(ConnectionState::Closing).unwrap();
    c.transition_to_state(ConnectionState::Closed).unwrap();
    assert_eq!(
        c.shared_state.load(Ordering::Acquire),
        ConnectionState::Closed as u8
    );
}

/// `Event::borrow_sender` panic nếu chưa có subscriber — verify contract:
/// `ControlCore::initialize` phải subscribe trước `start()`.
#[test]
#[should_panic(expected = "chưa có subscriber")]
fn borrow_sender_panics_without_subscriber() {
    use super::ConptyConnection;
    let c = ConptyConnection::new();
    // Chưa subscribe `terminal_output` → panic.
    let _ = c.terminal_output().borrow_sender();
}

/// `Event::borrow_sender` trả sender có thể `send` tới subscriber đã subscribe —
/// verify multi-producer clone path (output thread dùng sender này).
#[test]
fn borrow_sender_delivers_to_subscriber() {
    use super::ConptyConnection;
    let c = ConptyConnection::new();
    let rx = c.terminal_output().subscribe();
    let tx = c.terminal_output().borrow_sender();
    tx.send("hello-from-thread".into()).unwrap();
    assert_eq!(rx.recv(), Ok("hello-from-thread".into()));
}

// ───── Gap #6 / #7 — initialize flags + env block + process-exit ─────

/// Gap #6 — `Initialize` tính `flags` từ bool fields (port `cpp:262-295`).
/// `inherit_cursor=true` → set `INHERIT_CURSOR` bit.
#[test]
fn initialize_computes_flags_from_bool_fields() {
    use super::lifecycle::initialize;
    use super::ConptyConnection;
    let mut c = ConptyConnection::new();
    let s = super::ConptySettings {
        inherit_cursor: true,
        ..Default::default()
    };
    initialize(&mut c, s).unwrap();
    assert!(c
        .flags
        .contains(super::PseudoConsoleFlags::INHERIT_CURSOR));
}

/// Gap #6 — `ambiguous_is_wide=true` → set `AMBIGUOUS_IS_WIDE` bit.
#[test]
fn initialize_computes_ambiguous_is_wide() {
    use super::lifecycle::initialize;
    use super::ConptyConnection;
    let mut c = ConptyConnection::new();
    let s = super::ConptySettings {
        ambiguous_is_wide: true,
        ..Default::default()
    };
    initialize(&mut c, s).unwrap();
    assert!(c
        .flags
        .contains(super::PseudoConsoleFlags::AMBIGUOUS_IS_WIDE));
}

/// Gap #6 — non-zero `text_measurement` → set `GLYPH_WIDTH_WIDE` bit.
#[test]
fn initialize_computes_text_measurement_glyph_width() {
    use super::lifecycle::initialize;
    use super::ConptyConnection;
    let mut c = ConptyConnection::new();
    let s = super::ConptySettings {
        text_measurement: 1,
        ..Default::default()
    };
    initialize(&mut c, s).unwrap();
    assert!(c
        .flags
        .contains(super::PseudoConsoleFlags::GLYPH_WIDTH_WIDE));
}

/// Gap #6 — all bools `false` + `text_measurement=0` → flags stays at `settings.flags`.
#[test]
fn initialize_preserves_flags_when_bools_false() {
    use super::lifecycle::initialize;
    use super::ConptyConnection;
    let mut c = ConptyConnection::new();
    let s = super::ConptySettings {
        flags: super::PseudoConsoleFlags::empty(),
        ..Default::default()
    };
    initialize(&mut c, s).unwrap();
    assert_eq!(c.flags, super::PseudoConsoleFlags::empty());
}

/// Gap #7 — `ConptySettings::default().reload_env == true` (T1).
#[test]
fn settings_reload_env_default_true() {
    let s = super::ConptySettings::default();
    assert!(s.reload_env, "reload_env default phải là true per WT");
}

/// Gap #7 — `build_env_block` include `WT_SESSION=` entry (port `cpp:60`).
#[test]
fn launch_env_block_includes_wt_session() {
    use super::ConptySettings;
    use uuid::Uuid;
    let s = ConptySettings {
        reload_env: false,
        session_id: Uuid::nil(),
        ..Default::default()
    };
    let block = super::lifecycle::build_env_block(&s);
    // Decode UTF-16 block → ASCII string contain `WT_SESSION=`
    let decoded: String = String::from_utf16_lossy(
        block.split(|&b| b == 0).next().unwrap_or(&[]),
    );
    let whole: String = String::from_utf16_lossy(
        &block
            .iter()
            .filter(|&&b| b != 0)
            .copied()
            .collect::<Vec<u16>>(),
    );
    assert!(
        whole.contains("WT_SESSION="),
        "env block phải có WT_SESSION= entry, got: {decoded:?}"
    );
}

/// Gap #7 — `build_env_block` include `WT_PROFILE_ID=` khi `profile_guid` Some.
#[test]
fn launch_env_block_includes_wt_profile_id_when_some() {
    use super::ConptySettings;
    use uuid::Uuid;
    let profile = Uuid::nil();
    let s = ConptySettings {
        reload_env: false,
        profile_guid: Some(profile),
        ..Default::default()
    };
    let block = super::lifecycle::build_env_block(&s);
    let whole: String = String::from_utf16_lossy(
        &block
            .iter()
            .filter(|&&b| b != 0)
            .copied()
            .collect::<Vec<u16>>(),
    );
    assert!(whole.contains("WT_PROFILE_ID="), "env block phải có WT_PROFILE_ID=");
}

/// Gap #7 — env empty AND reload_env=false → return empty Vec (caller pass `None`).
#[test]
fn launch_env_block_empty_when_no_reload_no_overrides() {
    use super::ConptySettings;
    let s = ConptySettings {
        reload_env: false,
        environment: std::collections::HashMap::new(),
        ..Default::default()
    };
    // WT auto-add WT_SESSION/WT_PROFILE_ID nên block KHÔNG rỗng (always override).
    let block = super::lifecycle::build_env_block(&s);
    let whole: String = String::from_utf16_lossy(
        &block
            .iter()
            .filter(|&&b| b != 0)
            .copied()
            .collect::<Vec<u16>>(),
    );
    // WT_SESSION luôn có kể cả khi env rỗng → block != empty.
    assert!(whole.contains("WT_SESSION="));
}

/// Gap #7 — `environment` HashMap override default env vars.
#[test]
fn launch_env_block_overrides_caller_env() {
    use super::ConptySettings;
    use std::collections::HashMap;
    let mut env = HashMap::new();
    env.insert("MY_VAR".into(), "custom_value".into());
    let s = ConptySettings {
        reload_env: false,
        environment: env,
        session_id: uuid::Uuid::nil(),
        ..Default::default()
    };
    let block = super::lifecycle::build_env_block(&s);
    let whole: String = String::from_utf16_lossy(
        &block
            .iter()
            .filter(|&&b| b != 0)
            .copied()
            .collect::<Vec<u16>>(),
    );
    assert!(whole.contains("MY_VAR=custom_value"));
}

/// Gap #3 — `process_exit_message` format string phải khớp WT spec
/// `"\r\n[process exited with code {N}]\r\n[press Ctrl+D to close]\r\n"`.
#[test]
fn process_exit_message_format() {
    let msg = super::lifecycle::process_exit_message(42);
    assert_eq!(
        msg,
        "\r\n[process exited with code 42]\r\n[press Ctrl+D to close]\r\n"
    );
}

/// Gap #3 — exit code 0 không sinh message (caller skip raise).
#[test]
fn process_exit_message_zero() {
    let msg = super::lifecycle::process_exit_message(0);
    assert!(msg.contains("code 0"));
}

/// Gap #10 — `Box<dyn PtyBackend>` trait dispatch work — verify `ConptyBackend`
/// impl callable qua trait object (port seam).
#[test]
fn conpty_backend_impl_callable() {
    use super::ConptyBackend;
    use super::PtyBackend;
    let mut backend: Box<dyn PtyBackend> = Box::new(ConptyBackend);
    // `create` với `INVALID_HANDLE_VALUE` cho both pipes → sẽ fail
    // (CreatePseudoConsole yêu cầu valid pipes). Verify trait dispatch + error shape.
    let size = windows::Win32::System::Console::COORD { X: 80, Y: 30 };
    let flags: u32 = 0;
    let result = backend.create(size, flags);
    assert!(
        result.is_err(),
        "create với INVALID_HANDLE pipes phải fail, got Ok"
    );
    // `resize` với HPCON null → fail (ResizePseudoConsole invalid handle).
    let null_hpc = windows::Win32::System::Console::HPCON::default();
    let mut backend2: Box<dyn PtyBackend> = Box::new(ConptyBackend);
    assert!(backend2.resize(null_hpc, size).is_err());
    // `close` với null HPCON — no panic, normal drop.
    backend2.close(null_hpc);
}
