// SocketProtocol.swift
// Unix socket client for sending refresh requests to the daemon.

import Foundation
import Network

/// Client for communicating with the git-same daemon via Unix socket.
class SocketClient {
    private let socketPath: String

    init(socketPath: String = GitSameBadgeConstants.socketPath) {
        self.socketPath = socketPath
    }

    /// Send a command to the daemon and receive the response.
    func send(_ command: String, completion: @escaping (Swift.Result<String, Error>) -> Void) {
        let endpoint = NWEndpoint.unix(path: socketPath)
        let connection = NWConnection(to: endpoint, using: .tcp)

        connection.stateUpdateHandler = { state in
            switch state {
            case .ready:
                let message = "\(command)\n"
                let data = message.data(using: .utf8)!
                connection.send(content: data, completion: .contentProcessed { error in
                    if let error = error {
                        completion(.failure(error))
                        connection.cancel()
                        return
                    }
                    // Read response
                    connection.receive(minimumIncompleteLength: 1, maximumLength: 65536) { data, _, _, error in
                        if let error = error {
                            completion(.failure(error))
                        } else if let data = data, let response = String(data: data, encoding: .utf8) {
                            completion(.success(response))
                        } else {
                            completion(.success(""))
                        }
                        connection.cancel()
                    }
                })
            case .failed(let error):
                completion(.failure(error))
            default:
                break
            }
        }

        connection.start(queue: .global(qos: .utility))
    }

    /// Ping the daemon. Returns true if it responds.
    func ping(completion: @escaping (Bool) -> Void) {
        send("PING") { result in
            switch result {
            case .success(let response):
                completion(response.trimmingCharacters(in: .whitespacesAndNewlines) == "PONG")
            case .failure:
                completion(false)
            }
        }
    }

    /// Request a refresh of a specific path.
    func refresh(path: String, completion: @escaping (Bool) -> Void) {
        send("REFRESH \(path)") { result in
            completion(result.isSuccess)
        }
    }

    /// Request a full refresh.
    func refreshAll(completion: @escaping (Bool) -> Void) {
        send("REFRESH_ALL") { result in
            completion(result.isSuccess)
        }
    }
}

private extension Swift.Result {
    var isSuccess: Bool {
        switch self {
        case .success: return true
        case .failure: return false
        }
    }
}
