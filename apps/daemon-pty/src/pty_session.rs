//! PTY session: credit-based per-subscriber flow control.

use crate::messages::SpawnFrame;
use crate::replay::ReplayBuffer;
use crate::resolve::{resolve_command, BinaryNotFound};

use portable_pty::{native_pty_system, CommandBuilder, MasterPty, PtySize};
use std::collections::BTreeMap;
use std::collections::HashMap;
use std::io::{Read, Write};
use std::os::unix::io::RawFd;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Condvar, Mutex};
use std::time::{Duration, Instant};
use tokio::sync::mpsc::UnboundedSender;

/// The PTY transport behind a session: spawned by us, with portable-pty owning the master/child.
enum Pty {
    Spawned {
        master: Box<dyn MasterPty + Send>,
        writer: Box<dyn Write + Send>,
        child_killer: Box<dyn portable_pty::ChildKiller + Send + Sync>,
    },
}

pub const INITIAL_CREDIT: i64 = 65_536;
const DEFAULT_COLS: u16 = 220;
const DEFAULT_ROWS: u16 = 50;
pub const SHUTDOWN_GRACE_MS: u64 = 5_000;
/// SIGTERM grace before shutdown escalates to SIGKILL.
pub const SHUTDOWN_KILL_GRACE_MS: u64 = 250;

/// Terminal-plane status: the OS/process view of a session, distinct from the
/// agent's hook-derived status. Limited to `IDLE` | `WORKING` — terminal facts
/// cannot reliably establish `WAITING_INPUT`, which is a hook-only property.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum TermStatus {
    Idle,
    Working,
}

impl TermStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            TermStatus::Idle => "IDLE",
            TermStatus::Working => "WORKING",
        }
    }
}

fn derive_term_status(quiet: bool, foreground_pgrp: Option<i32>, root_pid: i32) -> TermStatus {
    if !quiet {
        return TermStatus::Working;
    }
    match foreground_pgrp {
        Some(fg) if fg != root_pid => TermStatus::Working,
        _ => TermStatus::Idle,
    }
}

pub enum SessionEvent {
    // `Arc<str>` for the data hot path: the reader thread clones a refcount per
    // chunk, not the id string.
    Data {
        session_id: Arc<str>,
        bytes: Vec<u8>,
    },
    Exit {
        session_id: String,
        code: Option<i32>,
        signal: Option<String>,
    },
}

struct ReadGate {
    paused: Mutex<bool>,
    cv: Condvar,
}

impl ReadGate {
    fn new() -> Self {
        Self {
            paused: Mutex::new(false),
            cv: Condvar::new(),
        }
    }
    fn set_paused(&self, value: bool) {
        let mut p = self.paused.lock().unwrap();
        *p = value;
        if !value {
            self.cv.notify_all();
        }
    }
    fn wait_while_paused(&self, stopped: &AtomicBool) {
        let mut p = self.paused.lock().unwrap();
        while *p && !stopped.load(Ordering::SeqCst) {
            p = self.cv.wait(p).unwrap();
        }
    }
}

struct Subscription {
    credit: i64,
    paused: bool,
}

pub struct Session {
    pub pid: u32,

    cur_cols: u16,
    cur_rows: u16,
    replay: ReplayBuffer,
    subscribers: HashMap<u64, Subscription>,

    // `term_status` is the last sampled value — the sampler emits only on transition.
    last_output_at: Instant,
    term_status: TermStatus,

    pty: Pty,

    gate: Arc<ReadGate>,
    stopped: Arc<AtomicBool>,
    pub killed_by_user: bool,
    pub exit_emitted: bool,
}

// `emit_exit_on_eof`: adopted sessions have no reaper thread — EOF on the master read is the exit.
fn start_reader(
    mut reader: Box<dyn Read + Send>,
    session_id: Arc<str>,
    gate: Arc<ReadGate>,
    stopped: Arc<AtomicBool>,
    tx: UnboundedSender<SessionEvent>,
    emit_exit_on_eof: bool,
) {
    std::thread::spawn(move || {
        let mut buf = [0u8; 65_536];
        loop {
            gate.wait_while_paused(&stopped);
            if stopped.load(Ordering::SeqCst) {
                break;
            }
            match reader.read(&mut buf) {
                Ok(0) => break,
                Ok(n) => {
                    if tx
                        .send(SessionEvent::Data {
                            session_id: Arc::clone(&session_id),
                            bytes: buf[..n].to_vec(),
                        })
                        .is_err()
                    {
                        break;
                    }
                }
                Err(_) => break,
            }
        }
        if emit_exit_on_eof {
            let _ = tx.send(SessionEvent::Exit {
                session_id: session_id.to_string(),
                code: None,
                signal: None,
            });
        }
    });
}

fn build_child_env(
    caller: &BTreeMap<String, String>,
    shell_fallback: &str,
) -> BTreeMap<String, String> {
    let get = |k: &str| std::env::var(k).ok();
    let mut env = BTreeMap::new();
    env.insert(
        "PATH".into(),
        get("PATH").unwrap_or_else(|| "/usr/local/bin:/usr/bin:/bin:/usr/sbin:/sbin".into()),
    );
    env.insert("HOME".into(), get("HOME").unwrap_or_default());
    env.insert("USER".into(), get("USER").unwrap_or_default());
    env.insert(
        "LOGNAME".into(),
        get("LOGNAME").or_else(|| get("USER")).unwrap_or_default(),
    );
    env.insert(
        "SHELL".into(),
        get("SHELL").unwrap_or_else(|| shell_fallback.to_string()),
    );
    env.insert(
        "LANG".into(),
        get("LANG").unwrap_or_else(|| "en_US.UTF-8".into()),
    );
    env.insert("TERM".into(), "xterm-256color".into());
    env.insert("COLORTERM".into(), "truecolor".into());
    if let Some(sock) = get("SSH_AUTH_SOCK") {
        env.insert("SSH_AUTH_SOCK".into(), sock);
    }
    for (k, v) in caller {
        env.insert(k.clone(), v.clone());
    }
    env
}

fn is_login_shell(command: &Option<String>, args: &[String], binary: &str) -> bool {
    if command.is_some() || !args.is_empty() {
        return false;
    }
    matches!(
        binary.rsplit('/').next(),
        Some("sh" | "bash" | "zsh" | "fish" | "dash" | "csh" | "tcsh" | "ksh")
    )
}

impl Session {
    pub fn spawn(
        frame: &SpawnFrame,
        events_tx: UnboundedSender<SessionEvent>,
    ) -> Result<Session, SpawnError> {
        let binary =
            resolve_command(frame.command.as_deref()).map_err(SpawnError::BinaryNotFound)?;

        let launch_args: Vec<String> = frame
            .args
            .iter()
            .filter(|a| !a.is_empty())
            .cloned()
            .collect();
        let caller_env = frame.env.clone().unwrap_or_default();
        let safe_env = build_child_env(&caller_env, &binary);

        let cols = if frame.cols == 0 {
            DEFAULT_COLS
        } else {
            frame.cols
        };
        let rows = if frame.rows == 0 {
            DEFAULT_ROWS
        } else {
            frame.rows
        };

        let pty_system = native_pty_system();
        let pair = pty_system
            .openpty(PtySize {
                rows,
                cols,
                pixel_width: 0,
                pixel_height: 0,
            })
            .map_err(|e| SpawnError::Spawn(e.to_string()))?;

        let mut cmd = CommandBuilder::new(&binary);
        if is_login_shell(&frame.command, &launch_args, &binary) {
            cmd.arg("-l");
        } else {
            for a in &launch_args {
                cmd.arg(a);
            }
        }
        cmd.cwd(&frame.cwd);
        cmd.env_clear();
        for (k, v) in &safe_env {
            cmd.env(k, v);
        }

        let mut child = pair
            .slave
            .spawn_command(cmd)
            .map_err(|e| SpawnError::Spawn(e.to_string()))?;
        drop(pair.slave);

        let pid = child.process_id().unwrap_or(0);
        let child_killer = child.clone_killer();

        let reader = pair
            .master
            .try_clone_reader()
            .map_err(|e| SpawnError::Spawn(e.to_string()))?;
        let writer = pair
            .master
            .take_writer()
            .map_err(|e| SpawnError::Spawn(e.to_string()))?;

        let gate = Arc::new(ReadGate::new());
        let stopped = Arc::new(AtomicBool::new(false));

        start_reader(
            reader,
            Arc::from(frame.session_id.as_str()),
            Arc::clone(&gate),
            Arc::clone(&stopped),
            events_tx.clone(),
            false,
        );

        {
            let session_id = frame.session_id.clone();
            let tx = events_tx;
            std::thread::spawn(move || {
                let status = child.wait();
                let (code, signal) = match status {
                    Ok(s) => exit_facts(&s),
                    Err(_) => (None, None),
                };
                let _ = tx.send(SessionEvent::Exit {
                    session_id,
                    code,
                    signal,
                });
            });
        }

        Ok(Session {
            pid,
            cur_cols: cols,
            cur_rows: rows,
            replay: ReplayBuffer::new(),
            subscribers: HashMap::new(),
            last_output_at: Instant::now(),
            term_status: TermStatus::Working,
            pty: Pty::Spawned {
                master: pair.master,
                writer,
                child_killer,
            },
            gate,
            stopped,
            killed_by_user: false,
            exit_emitted: false,
        })
    }

    pub fn write_input(&mut self, bytes: &[u8]) {
        let Pty::Spawned { writer, .. } = &mut self.pty;
        let _ = writer.write_all(bytes);
        let _ = writer.flush();
    }

    pub fn resize(&mut self, cols: u16, rows: u16) {
        self.cur_cols = cols;
        self.cur_rows = rows;
        let Pty::Spawned { master, .. } = &self.pty;
        let _ = master.resize(PtySize {
            rows,
            cols,
            pixel_width: 0,
            pixel_height: 0,
        });
    }

    pub fn replay_bytes(&self) -> Vec<u8> {
        self.replay.bytes()
    }

    pub fn append_replay(&mut self, bytes: &[u8]) {
        self.replay.push(bytes);
    }

    pub fn raw_master_fd(&self) -> Option<RawFd> {
        let Pty::Spawned { master, .. } = &self.pty;
        master.as_raw_fd()
    }

    // ── Terminal status ────────────────────────────────────────────────────

    pub fn mark_output(&mut self) {
        self.last_output_at = Instant::now();
    }

    // The child is its own session/group leader (setsid), so its pgid equals its pid.
    // Overflow maps to -1, which never equals a real foreground group.
    fn root_pgid(&self) -> i32 {
        i32::try_from(self.pid).unwrap_or(-1)
    }

    fn foreground_pgrp(&self) -> Option<i32> {
        let fd = self.raw_master_fd()?;
        // SAFETY: `tcgetpgrp` only reads the foreground process group of our own
        // valid, open PTY master fd; it has no other effects.
        #[allow(unsafe_code)]
        let pgrp = unsafe { nix::libc::tcgetpgrp(fd) };
        // `<= 0` covers both the error return and a `0` "no foreground group"
        // (which some platforms report for a master with no controlling child).
        (pgrp > 0).then_some(pgrp)
    }

    pub fn sample_term_status(&mut self, quiescence: Duration) -> Option<TermStatus> {
        let quiet = self.last_output_at.elapsed() >= quiescence;
        let next = derive_term_status(quiet, self.foreground_pgrp(), self.root_pgid());
        if next != self.term_status {
            self.term_status = next;
            Some(next)
        } else {
            None
        }
    }

    pub fn term_status(&self) -> TermStatus {
        self.term_status
    }

    pub fn has_subscribers(&self) -> bool {
        !self.subscribers.is_empty()
    }

    // ── Subscriber flow control ────────────────────────────────────────────

    pub fn add_subscriber(&mut self, conn_id: u64, initial_credit: i64) {
        self.subscribers.insert(
            conn_id,
            Subscription {
                credit: initial_credit,
                paused: false,
            },
        );
    }

    pub fn remove_subscriber(&mut self, conn_id: u64) {
        self.subscribers.remove(&conn_id);
        if self.subscribers.is_empty() {
            self.resume_read();
        }
    }

    pub fn subscriber_ids(&self) -> Vec<u64> {
        self.subscribers.keys().copied().collect()
    }

    pub fn add_credit(&mut self, conn_id: u64, bytes: i64) {
        if let Some(sub) = self.subscribers.get_mut(&conn_id) {
            let was_paused = sub.paused;
            sub.credit += bytes;
            if sub.paused && sub.credit > 0 {
                sub.paused = false;
            }
            if was_paused && !sub.paused {
                self.resume_read();
            }
        }
    }

    pub fn fan_out(&mut self, len: usize) -> Vec<u64> {
        let mut targets = Vec::new();
        for (id, sub) in self.subscribers.iter_mut() {
            if sub.paused {
                continue;
            }
            sub.credit -= len as i64;
            if sub.credit <= 0 {
                sub.credit = 0;
                sub.paused = true;
            }
            targets.push(*id);
        }
        if !self.subscribers.is_empty() && !self.any_active() {
            self.pause_read();
        }
        targets
    }

    fn any_active(&self) -> bool {
        self.subscribers.values().any(|s| !s.paused)
    }

    fn pause_read(&self) {
        self.gate.set_paused(true);
    }

    fn resume_read(&self) {
        self.gate.set_paused(false);
    }

    pub fn mark_killed_by_user(&mut self) {
        self.killed_by_user = true;
    }

    pub fn begin_kill(&mut self) {
        self.stopped.store(true, Ordering::SeqCst);
        self.gate.set_paused(false); // unpark the reader so it can exit
        send_signal(self.pid, Signal::Term);
        let pid = self.pid;
        let Pty::Spawned { child_killer, .. } = &self.pty;
        let mut killer = Some(child_killer.clone_killer());
        std::thread::spawn(move || {
            std::thread::sleep(std::time::Duration::from_millis(SHUTDOWN_GRACE_MS));
            send_signal(pid, Signal::Kill);
            if let Some(k) = killer.as_mut() {
                let _ = k.kill();
            }
        });
    }

    pub fn force_kill_now(&mut self) {
        self.stopped.store(true, Ordering::SeqCst);
        self.gate.set_paused(false);
        send_signal(self.pid, Signal::Term);
    }

    /// SIGKILL the child's process group and the PTY child, so a SIGTERM-ignoring
    /// child (e.g. an interactive login shell) is reaped rather than orphaned.
    pub fn hard_kill(&mut self) {
        self.stopped.store(true, Ordering::SeqCst);
        self.gate.set_paused(false);
        send_signal(self.pid, Signal::Kill);
        let Pty::Spawned { child_killer, .. } = &mut self.pty;
        let _ = child_killer.kill();
    }
}

#[derive(Debug)]
pub enum SpawnError {
    BinaryNotFound(BinaryNotFound),
    Spawn(String),
}

impl SpawnError {
    pub fn code(&self) -> &'static str {
        match self {
            SpawnError::BinaryNotFound(_) => "BinaryNotFound",
            SpawnError::Spawn(_) => "SpawnFailed",
        }
    }
    pub fn message(&self) -> String {
        match self {
            SpawnError::BinaryNotFound(e) => e.0.clone(),
            SpawnError::Spawn(m) => m.clone(),
        }
    }
}

enum Signal {
    Term,
    Kill,
}

fn send_signal(pid: u32, sig: Signal) {
    use nix::sys::signal::{kill, killpg, Signal as NixSignal};
    use nix::unistd::Pid;
    if pid == 0 {
        return;
    }
    let signum = match sig {
        Signal::Term => NixSignal::SIGTERM,
        Signal::Kill => NixSignal::SIGKILL,
    };
    let p = Pid::from_raw(pid as i32);
    let _ = killpg(p, signum);
    let _ = kill(p, signum);
}

// portable-pty only surfaces an exit code; signal death is encoded as 128+signo by convention.
fn exit_facts(status: &portable_pty::ExitStatus) -> (Option<i32>, Option<String>) {
    let code = status.exit_code() as i32;
    if (129..=159).contains(&code) {
        let signo = code - 128;
        let name = crate::signals::resolve_signal(
            crate::signals::SignalInput::Number(signo),
            crate::signals::SignalPlatform::host(),
        );
        if let crate::signals::ResolvedSignal::Known { name, .. } = name {
            return (Some(code), Some(name.to_string()));
        }
    }
    (Some(code), None)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn login_shell_detection() {
        assert!(is_login_shell(&None, &[], "/bin/zsh"));
        assert!(!is_login_shell(&Some("/bin/zsh".into()), &[], "/bin/zsh"));
        assert!(!is_login_shell(&None, &["-c".into()], "/bin/zsh"));
        assert!(!is_login_shell(&None, &[], "/usr/bin/htop"));
    }

    #[test]
    fn term_status_working_while_output_flows() {
        // Output within the quiescence window is WORKING regardless of foreground.
        assert_eq!(
            derive_term_status(false, Some(100), 100),
            TermStatus::Working
        );
        assert_eq!(
            derive_term_status(false, Some(200), 100),
            TermStatus::Working
        );
        assert_eq!(derive_term_status(false, None, 100), TermStatus::Working);
    }

    #[test]
    fn term_status_idle_when_quiet_and_root_holds_foreground() {
        // Shell at its prompt: root pid holds the terminal foreground, output quiet.
        assert_eq!(derive_term_status(true, Some(100), 100), TermStatus::Idle);
    }

    #[test]
    fn term_status_working_when_quiet_subjob_holds_foreground() {
        // Quiet sub-job (e.g. `sleep 10`, a silent compile): a job is running.
        assert_eq!(
            derive_term_status(true, Some(200), 100),
            TermStatus::Working
        );
    }

    #[test]
    fn term_status_degrades_to_idle_when_foreground_unreadable() {
        // No readable foreground group + quiet ⇒ degrade to quiescence ⇒ IDLE.
        assert_eq!(derive_term_status(true, None, 100), TermStatus::Idle);
    }

    fn spawn_test_session() -> Session {
        let (tx, _rx) = tokio::sync::mpsc::unbounded_channel();
        let frame = SpawnFrame {
            session_id: "term-test".into(),
            resume: None,
            command: None,
            args: vec![],
            env: None,
            cols: 80,
            rows: 24,
            cwd: "/".into(),
        };
        Session::spawn(&frame, tx).expect("spawn test session")
    }

    #[test]
    fn spawned_session_initial_status_is_working() {
        // The value a subscriber receives on subscribe, before any transition.
        let session = spawn_test_session();
        assert_eq!(session.term_status(), TermStatus::Working);
    }

    #[test]
    fn spawned_session_samples_idle_when_root_holds_foreground() {
        let mut session = spawn_test_session();
        // Wait for the child shell to claim the terminal foreground group; it is
        // its own session/group leader, so its pgid equals the session pid.
        let want = Some(session.pid as i32);
        for _ in 0..100 {
            if session.foreground_pgrp() == want {
                break;
            }
            std::thread::sleep(Duration::from_millis(5));
        }
        assert_eq!(session.foreground_pgrp(), want);
        // Forcing the quiet branch (zero threshold): root holds the foreground ⇒
        // Working -> Idle. Exercises the real master fd + derivation end to end.
        assert_eq!(
            session.sample_term_status(Duration::ZERO),
            Some(TermStatus::Idle)
        );
    }

    #[test]
    fn term_status_never_waiting_input() {
        // Across the whole input space, the value is only ever IDLE or WORKING.
        for quiet in [true, false] {
            for fg in [None, Some(100), Some(999)] {
                let s = derive_term_status(quiet, fg, 100);
                assert!(matches!(s, TermStatus::Idle | TermStatus::Working));
            }
        }
    }

    #[test]
    fn child_env_caller_overrides_base() {
        let mut caller = BTreeMap::new();
        caller.insert("TERM".into(), "dumb".into());
        caller.insert("MY_VAR".into(), "1".into());
        let env = build_child_env(&caller, "/bin/sh");
        assert_eq!(env.get("TERM").map(String::as_str), Some("dumb"));
        assert_eq!(env.get("MY_VAR").map(String::as_str), Some("1"));
        assert_eq!(env.get("COLORTERM").map(String::as_str), Some("truecolor"));
    }

    #[test]
    fn hard_kill_reaps_a_sigterm_ignoring_child() {
        use nix::sys::signal::kill;
        use nix::unistd::Pid;

        let (tx, _rx) = tokio::sync::mpsc::unbounded_channel();
        let frame = SpawnFrame {
            session_id: "kill-test".into(),
            resume: None,
            command: Some("/bin/sh".into()),
            args: vec![
                "-c".into(),
                "trap '' TERM; while :; do sleep 1; done".into(),
            ],
            env: None,
            cols: 80,
            rows: 24,
            cwd: "/".into(),
        };
        let mut session = Session::spawn(&frame, tx).expect("spawn test session");
        let pid = Pid::from_raw(session.pid as i32);

        // Let the child install its SIGTERM trap before signalling.
        std::thread::sleep(Duration::from_millis(200));

        session.force_kill_now();
        std::thread::sleep(Duration::from_millis(300));
        assert!(
            kill(pid, None).is_ok(),
            "child ignoring SIGTERM stays alive after force_kill_now"
        );

        session.hard_kill();
        let mut reaped = false;
        for _ in 0..100 {
            if kill(pid, None).is_err() {
                reaped = true;
                break;
            }
            std::thread::sleep(Duration::from_millis(50));
        }
        assert!(reaped, "child reaped after hard_kill");
    }
}
