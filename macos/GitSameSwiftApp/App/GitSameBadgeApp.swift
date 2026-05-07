// GitSameBadgeApp.swift
// Main entry point for the GitSameBadge host app.
// This is the seed for the future full macOS app.

import SwiftUI

@main
struct GitSameBadgeApp: App {
    @StateObject private var appState = AppState()

    var body: some Scene {
        WindowGroup {
            ContentView()
                .environmentObject(appState)
        }
        .windowResizability(.contentSize)
    }
}
