// AppState.swift
// Central observable state for the host app.

import Foundation
import SwiftUI

class AppState: ObservableObject {
    @Published var isDaemonRunning: Bool = false
    @Published var daemonPID: UInt32?
    @Published var lastScan: String?
    @Published var repoCount: Int = 0
    @Published var workspaces: [FinderWorkspaceInfo] = []

    private var refreshTimer: Timer?
    private let statusReader = StatusReader.shared

    init() {
        refresh()
        // Periodically refresh daemon status
        refreshTimer = Timer.scheduledTimer(withTimeInterval: 10, repeats: true) { [weak self] _ in
            self?.refresh()
        }
    }

    /// Refresh state from the status file.
    func refresh() {
        statusReader.reload()
        guard let status = statusReader.currentStatus else {
            isDaemonRunning = false
            daemonPID = nil
            lastScan = nil
            repoCount = 0
            workspaces = []
            return
        }

        daemonPID = status.daemonPid
        lastScan = status.timestamp
        repoCount = status.repos.count
        workspaces = status.workspaces

        // Check if daemon PID is alive
        isDaemonRunning = isProcessAlive(pid: status.daemonPid)
    }

    /// Start the daemon.
    func startDaemon() {
        let binaryPath = GitSameBadgeConstants.daemonBinaryPath
        let process = Process()
        process.executableURL = URL(fileURLWithPath: binaryPath)
        process.arguments = ["daemon", "--foreground"]
        process.standardOutput = FileHandle.nullDevice
        process.standardError = FileHandle.nullDevice

        do {
            try process.run()
            DispatchQueue.main.asyncAfter(deadline: .now() + 2) { [weak self] in
                self?.refresh()
            }
        } catch {
            // Failed to start daemon
        }
    }

    /// Stop the daemon.
    func stopDaemon() {
        guard let pid = daemonPID else { return }
        let process = Process()
        process.executableURL = URL(fileURLWithPath: "/bin/kill")
        process.arguments = ["-TERM", "\(pid)"]
        try? process.run()
        process.waitUntilExit()

        DispatchQueue.main.asyncAfter(deadline: .now() + 1) { [weak self] in
            self?.refresh()
        }
    }

    private func isProcessAlive(pid: UInt32) -> Bool {
        let process = Process()
        process.executableURL = URL(fileURLWithPath: "/bin/kill")
        process.arguments = ["-0", "\(pid)"]
        process.standardOutput = FileHandle.nullDevice
        process.standardError = FileHandle.nullDevice
        do {
            try process.run()
            process.waitUntilExit()
            return process.terminationStatus == 0
        } catch {
            return false
        }
    }

    deinit {
        refreshTimer?.invalidate()
    }
}
