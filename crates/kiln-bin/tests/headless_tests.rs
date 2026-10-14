use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use kiln_bin::{decode_loaded_file, parse_key_script, run_headless, EditorSession, SessionAction};
use kiln_input::{KeyCode, KeyEvent};
use kiln_services::file_io;

struct TestDir(PathBuf);

impl TestDir {
    fn new() -> Self {
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or(0, |duration| duration.as_nanos());
        let path =
            std::env::temp_dir().join(format!("kiln-headless-{}-{stamp}", std::process::id()));
        fs::create_dir_all(&path).expect("test directory creation succeeds");
        Self(path)
    }
}

impl Drop for TestDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

#[test]
fn scripted_insert_and_write_quit_edits_fixture() {
    let dir = TestDir::new();
    let path = dir.0.join("fixture.txt");
    fs::write(&path, b"hello").expect("fixture write succeeds");

    let session = run_headless(path.clone(), "i!<Esc>:wq<Enter>").expect("headless edit succeeds");

    assert!(session.exit_requested());
    assert_eq!(fs::read(path).expect("read succeeds"), b"!hello");
}

#[test]
fn dirty_quit_refuses_and_force_quit_discards() {
    let dir = TestDir::new();
    let path = dir.0.join("fixture.txt");
    fs::write(&path, b"hello").expect("fixture write succeeds");
    let bytes = file_io::read_file(&path).expect("load succeeds");
    let mut session = EditorSession::from_loaded(decode_loaded_file(path.clone(), &bytes));

    for key in parse_key_script("i!<Esc>:q<Enter>").expect("keys parse") {
        session.handle_key(key);
    }
    assert!(!session.exit_requested());
    assert!(session.buffer().is_dirty());

    for key in parse_key_script(":q!<Enter>").expect("keys parse") {
        session.handle_key(key);
    }
    assert!(session.exit_requested());
    assert_eq!(fs::read(path).expect("read succeeds"), b"hello");
}

#[test]
fn headless_can_create_a_new_file() {
    let dir = TestDir::new();
    let path = dir.0.join("new.txt");

    run_headless(path.clone(), "iHello<Esc>:wq<Enter>").expect("new file save succeeds");

    assert_eq!(fs::read(path).expect("new file exists"), b"Hello");
}

#[test]
fn binary_file_is_read_only_in_headless_session() {
    let dir = TestDir::new();
    let path = dir.0.join("binary.bin");
    fs::write(&path, b"a\0b").expect("binary fixture write succeeds");

    let session =
        run_headless(path.clone(), "iX<Esc>:wq<Enter>").expect("read-only edit is ignored");

    assert!(!session.exit_requested());
    assert!(session.buffer().is_read_only());
    assert_eq!(fs::read(path).expect("read succeeds"), b"a\0b");
}

#[test]
fn normal_quit_of_clean_buffer_exits() {
    let dir = TestDir::new();
    let path = dir.0.join("fixture.txt");
    fs::write(&path, b"hello").expect("fixture write succeeds");
    let session = run_headless(path, ":q<Enter>").expect("clean quit succeeds");
    assert!(session.exit_requested());
}

#[test]
fn parser_supports_literal_unicode_and_special_tokens() {
    assert_eq!(
        parse_key_script("iHi🙂<Esc>:wq<Enter>").expect("script parses"),
        vec![
            KeyEvent::char('i'),
            KeyEvent::char('H'),
            KeyEvent::char('i'),
            KeyEvent::char('🙂'),
            KeyEvent::new(KeyCode::Esc),
            KeyEvent::char(':'),
            KeyEvent::char('w'),
            KeyEvent::char('q'),
            KeyEvent::new(KeyCode::Enter),
        ]
    );
    assert!(parse_key_script("<Unknown>").is_err());
}

#[test]
fn direct_save_action_is_available_for_session_driver() {
    let dir = TestDir::new();
    let path = dir.0.join("fixture.txt");
    fs::write(&path, b"x").expect("fixture write succeeds");
    let bytes = file_io::read_file(&path).expect("load succeeds");
    let mut session = EditorSession::from_loaded(decode_loaded_file(path.clone(), &bytes));
    for key in parse_key_script(":w<Enter>").expect("keys parse") {
        if let SessionAction::Save(request) = session.handle_key(key) {
            assert_eq!(request.path, path);
            return;
        }
    }
    panic!("write command did not request a save");
}
