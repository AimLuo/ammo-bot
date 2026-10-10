//! CLI seam tests for `markplane serve` HTTPS / cert generation (TASK-kcivc).

use assert_cmd::cargo::cargo_bin;
use std::io::{BufRead, BufReader, Read};
use std::net::TcpListener;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};
use tempfile::TempDir;

fn free_port() -> u16 {
    TcpListener::bind("127.0.0.1:0")
        .unwrap()
        .local_addr()
        .unwrap()
        .port()
}

fn setup_project() -> TempDir {
    let tmp = TempDir::new().unwrap();
    Command::new(cargo_bin!("markplane"))
        .current_dir(tmp.path())
        .args(["init", "--name", "HTTPS Test", "--empty"])
        .output()
        .expect("init");
    tmp
}

struct ServeChild {
    child: Child,
    stdout: String,
}

impl Drop for ServeChild {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

#[allow(clippy::zombie_processes)] // ServeChild::drop always kill()+wait()
fn spawn_serve(
    project: &Path,
    xdg: &Path,
    port: u16,
    extra_args: &[&str],
) -> ServeChild {
    let mut child = Command::new(cargo_bin!("markplane"))
        .current_dir(project)
        .env("XDG_CONFIG_HOME", xdg)
        .args(["serve", "--port", &port.to_string()])
        .args(extra_args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn markplane serve");

    let stdout = child.stdout.take().expect("stdout");
    let mut reader = BufReader::new(stdout);
    let mut collected = String::new();
    let deadline = Instant::now() + Duration::from_secs(30);

    loop {
        if Instant::now() > deadline {
            let mut err = String::new();
            if let Some(mut stderr) = child.stderr.take() {
                let _ = stderr.read_to_string(&mut err);
            }
            let _ = child.kill();
            panic!(
                "serve did not become ready in time.\nstdout:\n{collected}\nstderr:\n{err}"
            );
        }

        let mut line = String::new();
        match reader.read_line(&mut line) {
            Ok(0) => {
                let mut err = String::new();
                if let Some(mut stderr) = child.stderr.take() {
                    let _ = stderr.read_to_string(&mut err);
                }
                panic!("serve exited early.\nstdout:\n{collected}\nstderr:\n{err}");
            }
            Ok(_) => {
                collected.push_str(&line);
                // Wait for the WebSocket line — last startup banner before bind.
                if collected.contains("WebSocket available at ") {
                    std::thread::sleep(Duration::from_millis(300));
                    return ServeChild {
                        child,
                        stdout: collected,
                    };
                }
            }
            Err(e) => panic!("read serve stdout: {e}"),
        }
    }
}

fn cert_paths(xdg: &Path) -> (PathBuf, PathBuf) {
    let dir = xdg.join("markplane").join("certs");
    (dir.join("cert.pem"), dir.join("key.pem"))
}

#[cfg(unix)]
fn mode(path: &Path) -> u32 {
    use std::os::unix::fs::PermissionsExt;
    std::fs::metadata(path).unwrap().permissions().mode() & 0o777
}

fn curl_get(url: &str, cacert: Option<&Path>) -> (i32, String) {
    let mut cmd = Command::new("curl");
    cmd.args(["-sS", "-o", "-", "-w", "%{http_code}", "--max-time", "5"]);
    if let Some(cert) = cacert {
        cmd.arg("--cacert").arg(cert);
    }
    cmd.arg(url);
    let output = cmd.output().expect("run curl");
    let body = String::from_utf8_lossy(&output.stdout).into_owned();
    // curl -w appends the status code after the body
    let status_str = body
        .chars()
        .rev()
        .take_while(|c| c.is_ascii_digit())
        .collect::<String>()
        .chars()
        .rev()
        .collect::<String>();
    let status: i32 = status_str.parse().unwrap_or(-1);
    let body_only = body
        .strip_suffix(&status_str)
        .unwrap_or(&body)
        .to_string();
    if !output.status.success() && status < 0 {
        let err = String::from_utf8_lossy(&output.stderr);
        panic!("curl failed for {url}: {err}\nstdout: {body}");
    }
    (status, body_only)
}

#[test]
fn serve_https_generates_cert_and_completes_handshake() {
    let project = setup_project();
    let xdg = TempDir::new().unwrap();
    let port = free_port();

    let serve = spawn_serve(project.path(), xdg.path(), port, &[]);
    assert!(
        serve.stdout.contains(&format!("https://localhost:{port}")),
        "expected https URL in stdout, got:\n{}",
        serve.stdout
    );
    assert!(
        serve.stdout.contains("wss://localhost:"),
        "expected wss URL, got:\n{}",
        serve.stdout
    );
    assert!(
        serve.stdout.contains("Trust this certificate")
            || serve.stdout.contains("security add-trusted-cert")
            || serve.stdout.contains("certutil"),
        "expected trust instructions on first start, got:\n{}",
        serve.stdout
    );

    let (cert, key) = cert_paths(xdg.path());
    assert!(cert.is_file(), "cert.pem missing");
    assert!(key.is_file(), "key.pem missing");
    #[cfg(unix)]
    {
        assert_eq!(mode(cert.parent().unwrap()), 0o700);
        assert_eq!(mode(&key), 0o600);
        assert_eq!(mode(&cert), 0o644);
    }

    let url = format!("https://localhost:{port}/api/summary");
    let (status, body) = curl_get(&url, Some(&cert));
    assert_eq!(status, 200, "body={body}");
    assert!(
        body.contains("HTTPS Test") || body.contains("\"name\""),
        "unexpected summary body: {body}"
    );
}

#[test]
fn serve_reuses_cert_without_reprinting_trust_on_second_start() {
    let project = setup_project();
    let xdg = TempDir::new().unwrap();
    let port1 = free_port();

    {
        let serve = spawn_serve(project.path(), xdg.path(), port1, &[]);
        assert!(serve.stdout.contains("Trust this certificate")
            || serve.stdout.contains("security add-trusted-cert")
            || serve.stdout.contains("certutil"));
    }

    let (cert, _) = cert_paths(xdg.path());
    let pem_before = std::fs::read(&cert).unwrap();

    let port2 = free_port();
    let serve = spawn_serve(project.path(), xdg.path(), port2, &[]);
    assert!(
        !serve.stdout.contains("Generated localhost TLS certificate"),
        "should reuse cert; stdout:\n{}",
        serve.stdout
    );
    let pem_after = std::fs::read(&cert).unwrap();
    assert_eq!(pem_before, pem_after);

    let url = format!("https://localhost:{port2}/api/summary");
    let (status, _) = curl_get(&url, Some(&cert));
    assert_eq!(status, 200);
}

#[test]
fn serve_dev_and_no_tls_use_http() {
    let project = setup_project();
    let xdg = TempDir::new().unwrap();

    let port_dev = free_port();
    let serve_dev = spawn_serve(project.path(), xdg.path(), port_dev, &["--dev"]);
    assert!(
        serve_dev
            .stdout
            .contains(&format!("http://localhost:{port_dev}")),
        "dev should print http URL:\n{}",
        serve_dev.stdout
    );
    assert!(
        !serve_dev.stdout.contains("https://"),
        "dev must not use https:\n{}",
        serve_dev.stdout
    );
    let (status, _) = curl_get(
        &format!("http://127.0.0.1:{port_dev}/api/summary"),
        None,
    );
    assert_eq!(status, 200);
    drop(serve_dev);

    // --dev should not create certs
    assert!(
        !xdg.path().join("markplane").join("certs").join("cert.pem").is_file(),
        "dev mode must not write certificates"
    );

    let port_plain = free_port();
    let serve_plain = spawn_serve(project.path(), xdg.path(), port_plain, &["--no-tls"]);
    assert!(
        serve_plain
            .stdout
            .contains(&format!("http://localhost:{port_plain}")),
        "--no-tls should print http URL:\n{}",
        serve_plain.stdout
    );
    assert!(
        serve_plain.stdout.contains("TLS disabled")
            || serve_plain.stdout.contains("--no-tls"),
        "expected --no-tls notice:\n{}",
        serve_plain.stdout
    );
    let (status, _) = curl_get(
        &format!("http://127.0.0.1:{port_plain}/api/summary"),
        None,
    );
    assert_eq!(status, 200);
}
