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
