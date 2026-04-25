//! Regression test: `visit_fullsized` targeting a panel inside an inner
//! sub-view tree correctly drives that panel to `viewed = true`.
//!
//! What this test asserts:
//! After the outer `visit` + `wait_for panel_viewed` sequence drives the
//! content sub-view's wrapper engine to cycle, a `visit_fullsized` targeting
//! the cosmos panel (identity `":"`) inside that sub-view tree causes the
//! cosmos panel's `viewed` flag to flip.  `wait_for panel_viewed` returns
//! `ok=true` — the inner SubView-scoped VisitingVAEngineClass cycles and
//! `RawVisitAbs` flips `viewed` correctly.
//!
//! History:
//! F015 calibration thought it had observed this path failing.  Reproduction
//! during F016 phase 1 falsified that — three deterministic runs show the
//! path works.  The test is kept as a regression check.  F016 was therefore
//! narrowed to "ship the emDirPanel cascade test using existing primitives"
//! (Phase 4 of the F016 plan).
//!
//! Gated `#[ignore]` because it requires a live display (X11 or Xvfb) and a
//! compiled `eaglemode` binary.

use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::UnixStream;
use std::path::PathBuf;
use std::process::{Child, Command};
use std::thread;
use std::time::{Duration, Instant};

// ---------------------------------------------------------------------------
// Harness helpers (independent copy — integration tests must not share code
// via module imports across test binaries)
// ---------------------------------------------------------------------------

fn spawn_and_connect() -> (Child, UnixStream) {
    let mut child = Command::new("cargo")
        .args(["run", "--bin", "eaglemode", "--quiet"])
        .env("EMCORE_DEBUG_CONTROL", "1")
        .env("EMCORE_DLOG", "1")
        .spawn()
        .expect("spawn eaglemode binary");
    let pid = child.id();
    let sock_path = PathBuf::from(format!("/tmp/eaglemode-rs.{}.sock", pid));

    let deadline = Instant::now() + Duration::from_secs(60);
    loop {
        if sock_path.exists() {
            if let Ok(s) = UnixStream::connect(&sock_path) {
                return (child, s);
            }
        }
        if Instant::now() > deadline {
            let _ = child.kill();
            panic!(
                "control socket did not appear within 60s at {:?}",
                sock_path
            );
        }
        thread::sleep(Duration::from_millis(200));
    }
}

/// Send one JSON line and read back the single-line reply.
fn send(s: &mut UnixStream, line: &str) -> String {
    writeln!(s, "{}", line).expect("write to control socket");
    let mut reader = BufReader::new(s.try_clone().expect("clone socket for read"));
    let mut buf = String::new();
    reader
        .read_line(&mut buf)
        .expect("read reply from control socket");
    buf
}

// ---------------------------------------------------------------------------
// Test
// ---------------------------------------------------------------------------

#[test]
#[ignore = "requires display + binary build"]
fn f016_visit_fullsized_drives_inner_subview_panel_to_viewed() {
    let (child, mut s) = spawn_and_connect();

    // Guard: kill the child even if we panic mid-test.
    struct KillOnDrop(Option<Child>);
    impl Drop for KillOnDrop {
        fn drop(&mut self) {
            if let Some(ref mut c) = self.0 {
                let _ = c.kill();
            }
        }
    }
    // Transfer ownership into the guard; we'll unwrap it for clean shutdown.
    let mut guard = KillOnDrop(Some(child));

    // Step 3 — let the StartupEngine settle.
    let reply = send(&mut s, r#"{"cmd":"wait_idle","timeout_ms":30000}"#);
    assert!(
        reply.contains("\"ok\":true"),
        "startup wait_idle failed: {}",
        reply
    );

    // Step 4 — visit the content-view SVP in the outer tree, then wait until
    // it is marked viewed.  This replicates the successful F010 sequence and
    // ensures the sub-view wrapper engine wakes up at least once.
    let reply = send(&mut s, r#"{"cmd":"visit","identity":"root:content view"}"#);
    assert!(
        reply.contains("\"ok\":true"),
        "visit content view failed: {}",
        reply
    );
    let reply = send(
        &mut s,
        r#"{"cmd":"wait_for","condition":{"kind":"panel_viewed","identity":"root:content view"},"timeout_ms":60000}"#,
    );
    assert!(
        reply.contains("\"ok\":true"),
        "wait_for panel_viewed (content view) failed: {}",
        reply
    );

    // Step 5 — let the inner sub-view's wrapper engine cycle at least once so
    // the cosmos panel materialises inside the sub-tree.
    let reply = send(&mut s, r#"{"cmd":"wait_idle","timeout_ms":5000}"#);
    assert!(
        reply.contains("\"ok\":true"),
        "post-visit wait_idle failed: {}",
        reply
    );

    // Step 6 — visit_fullsized targeting the cosmos panel (identity ":"
    // inside the content sub-view's tree).
    let reply = send(
        &mut s,
        r#"{"cmd":"visit_fullsized","view":"root:content view","identity":":"}"#,
    );
    assert!(
        reply.contains("\"ok\":true"),
        "visit_fullsized cosmos failed: {}",
        reply
    );

    // Step 7 — wait for the cosmos panel to become viewed.  With the inner
    // SubView-scoped VisitingVAEngineClass cycling correctly, 10 s is generous.
    let wait_reply = send(
        &mut s,
        r#"{"cmd":"wait_for","condition":{"kind":"panel_viewed","view":"root:content view","identity":":"},"timeout_ms":10000}"#,
    );

    // Step 8 — assert the positive outcome: visit_fullsized must drive the
    // cosmos panel to viewed.
    assert!(
        wait_reply.contains("\"ok\":true"),
        "wait_for panel_viewed (cosmos `:` inside `root:content view`) must succeed: {}",
        wait_reply
    );

    // Step 9 — dump the tree and assert the cosmos panel shows Viewed: yes.
    let dump_reply = send(&mut s, r#"{"cmd":"dump"}"#);
    assert!(
        dump_reply.contains("\"ok\":true"),
        "dump command failed: {}",
        dump_reply
    );
    let dump = std::fs::read_to_string("/tmp/debug.emTreeDump").expect("read /tmp/debug.emTreeDump");
    assert!(
        dump.contains("emVirtualCosmos::emVirtualCosmosPanel"),
        "dump must contain emVirtualCosmosPanel"
    );
    // Verify the cosmos panel record shows Viewed: yes.  The dump format writes
    // "Viewed: yes/no" within ~150 chars of the panel type-name header, so
    // a 500-char window is conservative.
    let cosmos_pos = dump
        .find("emVirtualCosmos::emVirtualCosmosPanel")
        .expect("emVirtualCosmosPanel position already asserted above");
    let window = &dump[cosmos_pos..cosmos_pos + 500.min(dump.len() - cosmos_pos)];
    assert!(
        window.contains("Viewed: yes"),
        "cosmos panel record must show Viewed: yes within 500 chars of its type-name header; \
         window was:\n{}",
        window
    );

    // Step 10 — clean shutdown.
    send(&mut s, r#"{"cmd":"quit"}"#);
    // Disarm the kill guard and wait for natural exit.
    let mut child = guard.0.take().expect("child still present");
    let _ = child.wait();
}
