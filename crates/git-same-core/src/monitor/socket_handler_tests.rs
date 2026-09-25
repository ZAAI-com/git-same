use super::*;
use crate::config::Config;
use crate::types::FinderStatus;
use tempfile::TempDir;
use tokio::sync::mpsc::{unbounded_channel, UnboundedReceiver};

#[test]
fn status_response_ends_with_newline() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("status.json");
    let writer = StatusFileWriter::new(path.clone());
    writer
        .write(&FinderStatus::new(0, "2026-06-21T00:00:00Z".to_string()))
        .unwrap();

    let resp = status_response(&writer);
    assert!(
        resp.ends_with('\n'),
        "Status response must end with newline"
    );
    assert_ne!(resp, "ERROR\n");
}

#[test]
fn status_response_error_when_missing() {
    let dir = TempDir::new().unwrap();
    let writer = StatusFileWriter::new(dir.path().join("does-not-exist.json"));
    let resp = status_response(&writer);
    assert_eq!(resp, "ERROR\n");
}

struct Harness {
    _dir: TempDir,
    live: LiveConfig,
    reload_tx: UnboundedSender<()>,
    scan_tx: UnboundedSender<ScanRequest>,
    scan_rx: UnboundedReceiver<ScanRequest>,
    status_writer: StatusFileWriter,
}

fn harness() -> Harness {
    let dir = TempDir::new().unwrap();
    let (reload_tx, _) = unbounded_channel();
    let (scan_tx, scan_rx) = unbounded_channel();
    let status_writer = StatusFileWriter::new(dir.path().join("status.json"));
    Harness {
        _dir: dir,
        live: LiveConfig::new(Config::default(), None),
        reload_tx,
        scan_tx,
        scan_rx,
        status_writer,
    }
}

/// Sends `request` through a connected pair and returns the handler's answer.
async fn exchange(h: &Harness, request: &str, timeout: Duration) -> String {
    let (server, mut client) = UnixStream::pair().unwrap();
    client.write_all(request.as_bytes()).await.unwrap();
    handle_socket_connection(
        server,
        &h.live,
        &h.reload_tx,
        &h.scan_tx,
        &h.status_writer,
        timeout,
    )
    .await;
    let mut answer = String::new();
    BufReader::new(client).read_line(&mut answer).await.unwrap();
    answer
}

#[tokio::test]
async fn refresh_all_is_answered_at_once_and_queues_one_full_scan() {
    let mut h = harness();
    let started = std::time::Instant::now();
    assert_eq!(exchange(&h, "REFRESH_ALL\n", CLIENT_TIMEOUT).await, "OK\n");
    assert!(started.elapsed() < Duration::from_secs(1));
    assert_eq!(h.scan_rx.try_recv().unwrap(), ScanRequest::Full);
    assert!(h.scan_rx.try_recv().is_err(), "exactly one request");
    assert!(
        !h.status_writer.path().exists(),
        "the handler never writes status.json"
    );
}

#[tokio::test]
async fn refresh_path_queues_the_canonical_repo() {
    let mut h = harness();
    let repo = TempDir::new().unwrap();
    let canonical = std::fs::canonicalize(repo.path()).unwrap();
    let request = format!("REFRESH {}\n", repo.path().display());
    assert_eq!(exchange(&h, &request, CLIENT_TIMEOUT).await, "OK\n");
    assert_eq!(h.scan_rx.try_recv().unwrap(), ScanRequest::Repo(canonical));
}

#[tokio::test]
async fn refresh_answers_error_once_the_loop_is_gone() {
    let (scan_tx, scan_rx) = unbounded_channel();
    drop(scan_rx);
    let h = Harness {
        scan_tx,
        ..harness()
    };
    assert_eq!(
        exchange(&h, "REFRESH_ALL\n", CLIENT_TIMEOUT).await,
        "ERROR\n"
    );
}

#[tokio::test]
async fn ping_still_answers_pong() {
    let mut h = harness();
    assert_eq!(exchange(&h, "PING\n", CLIENT_TIMEOUT).await, "PONG\n");
    assert!(h.scan_rx.try_recv().is_err());
}

#[tokio::test]
async fn a_client_that_sends_nothing_is_dropped() {
    let mut h = harness();
    let (server, client) = UnixStream::pair().unwrap();
    let started = std::time::Instant::now();
    handle_socket_connection(
        server,
        &h.live,
        &h.reload_tx,
        &h.scan_tx,
        &h.status_writer,
        Duration::from_millis(50),
    )
    .await;
    assert!(started.elapsed() < Duration::from_secs(2));
    assert!(h.scan_rx.try_recv().is_err());
    drop(client);
}
