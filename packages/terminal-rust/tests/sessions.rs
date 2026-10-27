use frame_terminal::{Output, SpawnOptions, Terminals};
use std::sync::{mpsc, Arc};
use std::time::{Duration, Instant};

#[test]
fn streams_output_and_rejects_another_owner() {
    let terminals = Terminals::default();
    let (send, receive) = mpsc::channel();
    let profile = terminals
        .profiles()
        .into_iter()
        .find(|profile| profile.id == if cfg!(windows) { "cmd" } else { "sh" })
        .expect("system shell");
    let options = SpawnOptions {
        profile: profile.id,
        cwd: None,
        cols: 80,
        rows: 24,
    };
    let id = terminals
        .spawn(
            "host-a",
            options,
            Arc::new(move |event| send.send(event).map_err(|error| error.to_string())),
        )
        .unwrap();
    assert!(terminals.write("host-b", &id, "echo WRONG\n").is_err());
    assert!(terminals.resize("host-b", &id, 100, 30).is_err());
    assert!(terminals.close("host-b", &id).is_err());
    terminals.resize("host-a", &id, 100, 30).unwrap();
    let command = if cfg!(windows) {
        "echo FRAME_ENGINE_OK\rexit\r"
    } else {
        "echo FRAME_ENGINE_OK\nexit\n"
    };
    terminals.write("host-a", &id, command).unwrap();
    let deadline = Instant::now() + Duration::from_secs(15);
    let mut bytes = Vec::new();
    loop {
        match receive
            .recv_timeout(deadline.saturating_duration_since(Instant::now()))
            .unwrap_or_else(|error| panic!("{error}; output={:?}", String::from_utf8_lossy(&bytes)))
        {
            Output::Data { data } => {
                bytes.extend(data);
                if bytes.windows(4).any(|chunk| chunk == b"\x1b[6n") {
                    let _ = terminals.write("host-a", &id, "\x1b[1;1R");
                }
            }
            Output::Exit { code } => {
                assert_eq!(code, 0);
                break;
            }
            Output::Error { message } => panic!("{message}"),
        }
    }
    assert!(String::from_utf8_lossy(&bytes).contains("FRAME_ENGINE_OK"));
    terminals.close("host-a", &id).unwrap();
}

#[test]
fn rejects_invalid_spawn_without_starting_processes() {
    let terminals = Terminals::default();
    assert!(terminals
        .spawn(
            "host",
            SpawnOptions {
                profile: "missing".into(),
                cwd: None,
                cols: 0,
                rows: 24,
            },
            Arc::new(|_| Ok(()))
        )
        .is_err());
}

#[test]
fn dropping_engine_terminates_owned_sessions() {
    let terminals = Terminals::default();
    let profile = terminals.profiles().into_iter().next().unwrap();
    let (send, receive) = mpsc::channel();
    terminals
        .spawn(
            "host",
            SpawnOptions {
                profile: profile.id,
                cwd: None,
                cols: 80,
                rows: 24,
            },
            Arc::new(move |event| send.send(event).map_err(|e| e.to_string())),
        )
        .unwrap();
    drop(terminals);
    let deadline = Instant::now() + Duration::from_secs(15);
    loop {
        if matches!(
            receive
                .recv_timeout(deadline.saturating_duration_since(Instant::now()))
                .unwrap(),
            Output::Exit { .. } | Output::Error { .. }
        ) {
            break;
        }
    }
}
