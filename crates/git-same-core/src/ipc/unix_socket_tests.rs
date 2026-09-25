use super::*;

#[test]
fn test_parse_command_ping() {
    assert_eq!(DaemonCommand::parse("PING"), DaemonCommand::Ping);
    assert_eq!(DaemonCommand::parse("PING\n"), DaemonCommand::Ping);
}

#[test]
fn test_parse_command_refresh() {
    assert_eq!(
        DaemonCommand::parse("REFRESH /path/to/repo"),
        DaemonCommand::Refresh(PathBuf::from("/path/to/repo"))
    );
}

#[test]
fn test_parse_command_refresh_all() {
    assert_eq!(
        DaemonCommand::parse("REFRESH_ALL"),
        DaemonCommand::RefreshAll
    );
}

#[test]
fn test_parse_command_status() {
    assert_eq!(DaemonCommand::parse("STATUS"), DaemonCommand::Status);
}

#[test]
fn test_parse_command_unknown() {
    assert_eq!(
        DaemonCommand::parse("FOOBAR"),
        DaemonCommand::Unknown("FOOBAR".to_string())
    );
}

#[test]
fn test_parse_command_refresh_with_spaces_in_path() {
    assert_eq!(
        DaemonCommand::parse("REFRESH /path/to/my repo"),
        DaemonCommand::Refresh(PathBuf::from("/path/to/my repo"))
    );
}

#[test]
fn test_parse_command_refresh_preserves_leading_space_in_path() {
    // The path argument is no longer inner-trimmed, so whitespace that is
    // part of the path (after the single "REFRESH " delimiter) is preserved.
    assert_eq!(
        DaemonCommand::parse("REFRESH  /leading-space"),
        DaemonCommand::Refresh(PathBuf::from(" /leading-space"))
    );
}

#[tokio::test]
async fn test_socket_listener_bind_and_cleanup() {
    let temp = tempfile::tempdir().unwrap();
    let sock_path = temp.path().join("test.sock");
    let listener = UnixSocketListener::new(sock_path.clone());

    // Bind should succeed
    let _tokio_listener = listener.bind().await.unwrap();
    assert!(sock_path.exists());

    // Cleanup should remove the socket
    listener.cleanup();
    assert!(!sock_path.exists());
}

#[tokio::test]
async fn test_socket_listener_removes_stale_socket() {
    let temp = tempfile::tempdir().unwrap();
    let sock_path = temp.path().join("test.sock");

    // Create a stale socket file
    std::fs::write(&sock_path, "stale").unwrap();
    assert!(sock_path.exists());

    let listener = UnixSocketListener::new(sock_path.clone());
    let _tokio_listener = listener.bind().await.unwrap();

    // Should have removed the stale file and created a real socket
    assert!(sock_path.exists());
    listener.cleanup();
}

#[tokio::test]
async fn test_socket_client_server_roundtrip() {
    let temp = tempfile::tempdir().unwrap();
    let sock_path = temp.path().join("test.sock");

    let listener = UnixSocketListener::new(sock_path.clone());
    let tokio_listener = listener.bind().await.unwrap();

    // Spawn a simple server that responds to PING
    let server = tokio::spawn(async move {
        let (stream, _) = tokio_listener.accept().await.unwrap();
        let mut reader = BufReader::new(stream);
        let mut line = String::new();
        reader.read_line(&mut line).await.unwrap();

        let cmd = DaemonCommand::parse(&line);
        assert_eq!(cmd, DaemonCommand::Ping);

        let stream = reader.into_inner();
        let mut stream = stream;
        write_response(&mut stream, "PONG\n").await.unwrap();
    });

    // Client sends PING
    let client = UnixSocketClient::new(sock_path);
    let is_alive = client.ping().await;
    assert!(is_alive);

    server.await.unwrap();
    listener.cleanup();
}

#[test]
fn test_display_renders_the_wire_text_parse_reads() {
    for cmd in [
        DaemonCommand::Refresh(PathBuf::from("/path/to/my repo")),
        DaemonCommand::RefreshAll,
        DaemonCommand::Status,
        DaemonCommand::Ping,
        DaemonCommand::Unknown("FOOBAR".to_string()),
    ] {
        assert_eq!(DaemonCommand::parse(&cmd.to_string()), cmd);
    }
    assert_eq!(DaemonCommand::RefreshAll.to_string(), "REFRESH_ALL");
}

/// Timeout for the "busy monitor" tests. Real callers wait seconds.
const SHORT: Duration = Duration::from_millis(100);

/// Upper bound for a call made with [`SHORT`], generous for a slow CI host.
const PROMPTLY: Duration = Duration::from_secs(5);

#[tokio::test]
async fn test_request_is_pending_when_the_monitor_never_accepts() {
    let temp = tempfile::tempdir().unwrap();
    let sock_path = temp.path().join("busy.sock");
    // Bound and listening, but never accepting: a monitor stuck in a scan.
    let _listener = UnixSocketListener::new(sock_path.clone())
        .bind()
        .await
        .unwrap();
    let client = UnixSocketClient::new(sock_path);

    let started = std::time::Instant::now();
    assert_eq!(client.request("PING", SHORT).await.unwrap(), Reply::Pending);
    client.notify("REFRESH_ALL", SHORT).await.unwrap();
    assert!(started.elapsed() < PROMPTLY, "took {:?}", started.elapsed());
}

#[tokio::test]
async fn test_request_is_pending_when_the_monitor_never_answers() {
    let temp = tempfile::tempdir().unwrap();
    let sock_path = temp.path().join("mute.sock");
    let tokio_listener = UnixSocketListener::new(sock_path.clone())
        .bind()
        .await
        .unwrap();

    // Accepts and reads the command, then holds the connection open without
    // answering: an older monitor running a full scan before its "OK".
    let server = tokio::spawn(async move {
        let (stream, _) = tokio_listener.accept().await.unwrap();
        let mut reader = BufReader::new(stream);
        let mut line = String::new();
        reader.read_line(&mut line).await.unwrap();
        std::future::pending::<()>().await;
    });

    let client = UnixSocketClient::new(sock_path);
    let started = std::time::Instant::now();
    assert_eq!(
        client.request("REFRESH_ALL", SHORT).await.unwrap(),
        Reply::Pending
    );
    assert!(started.elapsed() < PROMPTLY, "took {:?}", started.elapsed());
    server.abort();
}

/// A nudge that gives up before the monitor accepts must still be delivered:
/// the monitor reads the command once it gets to the connection.
#[tokio::test]
async fn test_a_command_written_before_accept_survives_the_client_leaving() {
    let temp = tempfile::tempdir().unwrap();
    let sock_path = temp.path().join("late.sock");
    let tokio_listener = UnixSocketListener::new(sock_path.clone())
        .bind()
        .await
        .unwrap();

    UnixSocketClient::new(sock_path)
        .notify("REFRESH_ALL", SHORT)
        .await
        .unwrap();

    let (stream, _) = tokio_listener.accept().await.unwrap();
    let mut reader = BufReader::new(stream);
    let mut line = String::new();
    reader.read_line(&mut line).await.unwrap();
    assert_eq!(DaemonCommand::parse(&line), DaemonCommand::RefreshAll);
}

#[tokio::test]
async fn test_request_to_a_missing_monitor_is_an_error() {
    let temp = tempfile::tempdir().unwrap();
    let client = UnixSocketClient::new(temp.path().join("absent.sock"));

    assert!(client.request("PING", SHORT).await.is_err());
    assert!(client.notify("REFRESH_ALL", SHORT).await.is_err());
}

#[tokio::test]
async fn test_request_returns_an_empty_answer_when_the_monitor_hangs_up() {
    let temp = tempfile::tempdir().unwrap();
    let sock_path = temp.path().join("hangup.sock");
    let tokio_listener = UnixSocketListener::new(sock_path.clone())
        .bind()
        .await
        .unwrap();

    // Reads the command, then drops the stream without answering.
    let server = tokio::spawn(async move {
        let (stream, _) = tokio_listener.accept().await.unwrap();
        let mut reader = BufReader::new(stream);
        let mut line = String::new();
        reader.read_line(&mut line).await.unwrap();
    });

    let reply = UnixSocketClient::new(sock_path)
        .request("REFRESH_ALL", PROMPTLY)
        .await
        .unwrap();
    assert_eq!(reply, Reply::Answered(String::new()));
    server.await.unwrap();
}

#[tokio::test]
async fn test_request_returns_the_answer_line() {
    let temp = tempfile::tempdir().unwrap();
    let sock_path = temp.path().join("ok.sock");
    let tokio_listener = UnixSocketListener::new(sock_path.clone())
        .bind()
        .await
        .unwrap();

    let server = tokio::spawn(async move {
        let (stream, _) = tokio_listener.accept().await.unwrap();
        let mut reader = BufReader::new(stream);
        let mut line = String::new();
        reader.read_line(&mut line).await.unwrap();
        write_response(reader.get_mut(), "OK\n").await.unwrap();
        line
    });

    let reply = UnixSocketClient::new(sock_path)
        .request("REFRESH /tmp/x", PROMPTLY)
        .await
        .unwrap();
    assert_eq!(reply, Reply::Answered("OK\n".to_string()));
    assert_eq!(server.await.unwrap(), "REFRESH /tmp/x\n");
}

#[tokio::test]
async fn test_send_gives_up_instead_of_hanging() {
    assert!(SEND_TIMEOUT > NUDGE_TIMEOUT);

    let temp = tempfile::tempdir().unwrap();
    let sock_path = temp.path().join("stuck.sock");
    let _listener = UnixSocketListener::new(sock_path.clone())
        .bind()
        .await
        .unwrap();
    let client = UnixSocketClient::new(sock_path);

    let started = std::time::Instant::now();
    assert!(client.send_within("PING", SHORT).await.is_err());
    assert!(started.elapsed() < PROMPTLY, "took {:?}", started.elapsed());
}
