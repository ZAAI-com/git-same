// ContentView.swift
// Main window content for the GitSameBadge host app.

import SwiftUI

struct ContentView: View {
    @EnvironmentObject var appState: AppState

    var body: some View {
        NavigationSplitView {
            // Sidebar
            List {
                Section("Status") {
                    NavigationLink(destination: DaemonStatusView()) {
                        Label("Daemon", systemImage: "server.rack")
                    }
                }

                Section("Workspaces") {
                    ForEach(appState.workspaces, id: \.name) { workspace in
                        NavigationLink(destination: WorkspaceDetailView(workspace: workspace)) {
                            Label(workspace.name, systemImage: "folder")
                        }
                    }
                }

                Section("Settings") {
                    NavigationLink(destination: SettingsView()) {
                        Label("Preferences", systemImage: "gear")
                    }
                }
            }
            .listStyle(.sidebar)
            .frame(minWidth: 180)
        } detail: {
            DaemonStatusView()
        }
        .frame(minWidth: 600, minHeight: 400)
    }
}

struct DaemonStatusView: View {
    @EnvironmentObject var appState: AppState

    var body: some View {
        VStack(alignment: .leading, spacing: 16) {
            // Daemon status header
            HStack {
                Circle()
                    .fill(appState.isDaemonRunning ? Color.green : Color.red)
                    .frame(width: 12, height: 12)
                Text(appState.isDaemonRunning ? "Daemon Running" : "Daemon Stopped")
                    .font(.title2)
                    .fontWeight(.semibold)
            }

            if let pid = appState.daemonPID, appState.isDaemonRunning {
                LabeledContent("PID", value: "\(pid)")
            }
            if let lastScan = appState.lastScan {
                LabeledContent("Last Scan", value: lastScan)
            }
            LabeledContent("Repos Monitored", value: "\(appState.repoCount)")

            Divider()

            // Actions
            HStack {
                if appState.isDaemonRunning {
                    Button("Stop Daemon") {
                        appState.stopDaemon()
                    }
                    Button("Refresh") {
                        appState.refresh()
                    }
                } else {
                    Button("Start Daemon") {
                        appState.startDaemon()
                    }
                }
            }

            Spacer()

            // Extension status hint
            Text("To enable the Finder extension, go to:")
                .font(.caption)
                .foregroundColor(.secondary)
            Text("System Settings > Privacy & Security > Extensions > Finder")
                .font(.caption)
                .foregroundColor(.secondary)
        }
        .padding()
        .frame(maxWidth: .infinity, alignment: .leading)
    }
}

struct WorkspaceDetailView: View {
    let workspace: FinderWorkspaceInfo

    var body: some View {
        VStack(alignment: .leading, spacing: 12) {
            Text(workspace.name)
                .font(.title2)
                .fontWeight(.semibold)

            LabeledContent("Root", value: workspace.root)
            LabeledContent("Organizations", value: workspace.orgs.joined(separator: ", "))

            Spacer()
        }
        .padding()
        .frame(maxWidth: .infinity, alignment: .leading)
    }
}

struct SettingsView: View {
    var body: some View {
        VStack(alignment: .leading, spacing: 16) {
            Text("Preferences")
                .font(.title2)
                .fontWeight(.semibold)

            Text("Settings will be available in a future update.")
                .foregroundColor(.secondary)

            Spacer()
        }
        .padding()
        .frame(maxWidth: .infinity, alignment: .leading)
    }
}
