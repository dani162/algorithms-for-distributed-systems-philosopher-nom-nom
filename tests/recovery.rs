use std::io::Read;
use std::process::{Child, Command, Stdio};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

struct Proc {
    child: Child,
}

impl Proc {
    fn kill_and_wait(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

impl Drop for Proc {
    fn drop(&mut self) {
        self.kill_and_wait();
    }
}

fn spawn_bin(
    name: &'static str,
    args: &[&str],
    envs: &[(&str, &str)],
    logs: Arc<Mutex<String>>,
) -> Proc {
    let path = format!("./target/release/{}", name);

    let mut cmd = Command::new(path);
    cmd.args(args).stdout(Stdio::piped()).stderr(Stdio::piped());

    for (k, v) in envs {
        cmd.env(k, v);
    }

    let mut child = cmd
        .spawn()
        .unwrap_or_else(|e| panic!("spawn {} failed: {}", name, e));

    let mut out = child.stdout.take().unwrap();
    let mut err = child.stderr.take().unwrap();

    let logs_out = logs.clone();
    thread::spawn(move || {
        let mut buf = Vec::new();
        let _ = out.read_to_end(&mut buf);
        let s = String::from_utf8_lossy(&buf);
        let mut l = logs_out.lock().unwrap();
        l.push_str(&format!("[{} stdout]\n{}\n", name, s));
    });

    let logs_err = logs.clone();
    thread::spawn(move || {
        let mut buf = Vec::new();
        let _ = err.read_to_end(&mut buf);
        let s = String::from_utf8_lossy(&buf);
        let mut l = logs_err.lock().unwrap();
        l.push_str(&format!("[{} stderr]\n{}\n", name, s));
    });

    Proc { child }
}

#[test]
fn token_recovery_only_once() {
    let status = Command::new("cargo")
        .args(["build", "--release"])
        .status()
        .expect("failed to run cargo build --release");
    assert!(status.success(), "build failed");

    let logs = Arc::new(Mutex::new(String::new()));

    let envs = [("RUST_LOG", "info"), ("DROP_TOKEN_PCT", "70")];

    let mut procs: Vec<Proc> = Vec::new();

    procs.push(spawn_bin(
        "init",
        &["127.0.0.1:3333", "--thinker", "5", "--tokens", "1"],
        &envs,
        logs.clone(),
    ));

    thread::sleep(Duration::from_millis(500));

    for _ in 0..5 {
        procs.push(spawn_bin(
            "thinker",
            &["0.0.0.0:0", "--init-server", "127.0.0.1:3333"],
            &envs,
            logs.clone(),
        ));
    }

    for _ in 0..5 {
        procs.push(spawn_bin(
            "fork",
            &["0.0.0.0:0", "--init-server", "127.0.0.1:3333"],
            &envs,
            logs.clone(),
        ));
    }

    thread::sleep(Duration::from_secs(40));

    for p in procs.iter_mut() {
        p.kill_and_wait();
    }

    let log_text = logs.lock().unwrap().clone();
    let regen_count = log_text.matches("Token timeout -> regenerated").count();

    assert!(
        regen_count <= 1,
        "expected <= 1 token regeneration, got {}\n\n{}",
        regen_count,
        log_text
    );
}
