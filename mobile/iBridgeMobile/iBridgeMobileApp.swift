import SwiftUI

@main
struct iBridgeMobileApp: App {
    @StateObject private var model = MobileModel()

    var body: some Scene {
        WindowGroup {
            ContentView(model: model)
        }
    }
}
