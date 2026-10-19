// SocketProtocol.swift
// Unix socket client for sending refresh requests to the monitor.

import Foundation
import Network

/// Errors raised by `SocketClient` itself (Network.framework errors pass through).
enum SocketClientError: Error {
    /// The monitor did not answer within the request's time limit.
    case timedOut
}

/// Client for communicating with the git-same monitor via Unix socket.
///
/// Every request completes exactly once: with the monitor's answer line, with
/// an error, or with `SocketClientError.timedOut` after `timeout` seconds.
/// The monitor answers REFRESH and REFRESH_ALL as soon as they are queued, so
/// a request that runs out of time means a busy or stuck monitor.
class SocketClient {
    /// Default time limit for one request, matching `gisa refresh`.
    static let defaultTimeout: TimeInterval = 5

    private let socketPath: String
    /// Serializes every connection callback and timer of this client, so the
    /// per-request state below is never touched from two threads at once.
    private let queue = DispatchQueue(label: "com.zaai.git-same.socket-client", qos: .utility)

    init(socketPath: String = GitSameBadgesConstants.socketPath) {
        self.socketPath = socketPath
    }

    /// Send a command to the monitor and receive its one-line response.
    func send(
        _ command: String,
        timeout: TimeInterval = SocketClient.defaultTimeout,
        completion: @escaping (Swift.Result<String, Error>) -> Void
    ) {
        let connection = NWConnection(to: NWEndpoint.unix(path: socketPath), using: .tcp)
        let request = Request(connection: connection, completion: completion)

        connection.stateUpdateHandler = { state in
            switch state {
            case .ready:
                request.sendAndReceive("\(command)\n")
            case .waiting(let error), .failed(let error):
                // A missing socket or a refused connection lands in `.waiting`,
                // where Network.framework would otherwise wait indefinitely.
                request.finish(.failure(error))
            default:
                break
            }
        }

        let timer = DispatchWorkItem { request.finish(.failure(SocketClientError.timedOut)) }
        request.timer = timer
        queue.asyncAfter(deadline: .now() + timeout, execute: timer)
        connection.start(queue: queue)
    }

    /// Ping the monitor. Returns true if it responds.
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

/// State of one request. Only ever touched on the owning client's queue.
private final class Request {
    private let connection: NWConnection
    private var completion: ((Swift.Result<String, Error>) -> Void)?
    private var received = Data()
    var timer: DispatchWorkItem?

    init(connection: NWConnection, completion: @escaping (Swift.Result<String, Error>) -> Void) {
        self.connection = connection
        self.completion = completion
    }

    func sendAndReceive(_ message: String) {
        connection.send(content: Data(message.utf8), completion: .contentProcessed { error in
            if let error = error {
                self.finish(.failure(error))
            } else {
                self.receiveLine()
            }
        })
    }

    /// Reads until the monitor's newline-terminated answer, or until it hangs up.
    private func receiveLine() {
        connection.receive(minimumIncompleteLength: 1, maximumLength: 65536) { data, _, isComplete, error in
            if let data = data {
                self.received.append(data)
            }
            if let error = error {
                self.finish(.failure(error))
            } else if isComplete || self.received.contains(UInt8(ascii: "\n")) {
                self.finish(.success(String(decoding: self.received, as: UTF8.self)))
            } else {
                self.receiveLine()
            }
        }
    }

    /// Completes the request once; later calls (a late reply, a state change
    /// after cancel, the timer) are ignored.
    func finish(_ result: Swift.Result<String, Error>) {
        guard let completion = completion else { return }
        self.completion = nil
        timer?.cancel()
        timer = nil
        // Break the handler -> connection -> handler cycle so nothing leaks.
        connection.stateUpdateHandler = nil
        connection.cancel()
        completion(result)
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
