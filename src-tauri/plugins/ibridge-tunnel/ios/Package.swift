// swift-tools-version:5.9
import PackageDescription

let package = Package(
  name: "tauri-plugin-ibridge-tunnel",
  platforms: [.iOS(.v17)],
  products: [
    .library(
      name: "tauri-plugin-ibridge-tunnel",
      type: .static,
      targets: ["tauri-plugin-ibridge-tunnel"])
  ],
  dependencies: [
    .package(name: "Tauri", path: "../.tauri/tauri-api")
  ],
  targets: [
    .target(
      name: "tauri-plugin-ibridge-tunnel",
      dependencies: [.byName(name: "Tauri")],
      path: "Sources")
  ]
)
