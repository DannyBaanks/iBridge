import SwiftUI
import UniformTypeIdentifiers

struct ContentView: View {
    @ObservedObject var model: MobileModel
    @State private var importingIPA = false
    @State private var credential = ""

    private var ipaType: UTType {
        UTType(filenameExtension: "ipa") ?? .data
    }

    var body: some View {
        NavigationStack {
            ScrollView {
                VStack(spacing: 18) {
                    header
                    accountCard
                    actionCard
                    statusCard
                }
                .padding(20)
            }
            .navigationTitle("iBridge")
            .navigationBarTitleDisplayMode(.inline)
            .fileImporter(
                isPresented: $importingIPA,
                allowedContentTypes: [ipaType],
                allowsMultipleSelection: false
            ) { result in
                guard case .success(let urls) = result, let url = urls.first else { return }
                Task { await model.installImportedIPA(url) }
            }
        }
    }

    private var header: some View {
        VStack(spacing: 8) {
            Image(systemName: "point.3.connected.trianglepath.dotted")
                .font(.system(size: 44, weight: .semibold))
                .symbolRenderingMode(.hierarchical)
            Text("Firma y refresca desde este iPhone")
                .font(.headline)
                .multilineTextAlignment(.center)
            Text("El pairing y la cuenta llegan automáticamente desde iBridge Desktop.")
                .font(.subheadline)
                .foregroundStyle(.secondary)
                .multilineTextAlignment(.center)
        }
        .frame(maxWidth: .infinity)
        .padding(.vertical, 8)
    }

    private var accountCard: some View {
        VStack(alignment: .leading, spacing: 12) {
            Label("Cuenta", systemImage: "person.crop.circle")
                .font(.headline)

            if let configuration = model.configuration {
                LabeledContent("Apple ID", value: configuration.appleID)
                LabeledContent("iPhone", value: configuration.deviceName)
                LabeledContent("iOS", value: configuration.deviceVersion)
            } else {
                Text("Instala iBridge Mobile una vez desde iBridge Desktop para completar el bootstrap.")
                    .foregroundStyle(.secondary)
            }

            if model.needsCredential {
                SecureField("Contraseña de Apple ID", text: $credential)
                    .textContentType(.password)
                    .textFieldStyle(.roundedBorder)
                Button("Guardar en Keychain") {
                    model.saveCredential(credential)
                    credential = ""
                }
                .buttonStyle(.bordered)
            }

            if model.needsTwoFactor {
                TextField("Código 2FA", text: $model.verificationCode)
                    .keyboardType(.numberPad)
                    .textContentType(.oneTimeCode)
                    .textFieldStyle(.roundedBorder)
                Button("Continuar con 2FA") {
                    Task { await model.submitTwoFactor() }
                }
                .buttonStyle(.borderedProminent)
                .disabled(model.busy || model.verificationCode.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty)
            }
        }
        .frame(maxWidth: .infinity, alignment: .leading)
        .padding(16)
        .background(.thinMaterial, in: RoundedRectangle(cornerRadius: 18))
    }

    private var actionCard: some View {
        VStack(spacing: 12) {
            Button {
                importingIPA = true
            } label: {
                Label("Seleccionar IPA e instalar", systemImage: "square.and.arrow.down")
                    .frame(maxWidth: .infinity)
            }
            .buttonStyle(.borderedProminent)
            .controlSize(.large)
            .disabled(model.busy || model.configuration == nil)

            HStack(spacing: 10) {
                Button {
                    Task { await model.refreshAll() }
                } label: {
                    Label("Refrescar apps", systemImage: "arrow.clockwise")
                        .frame(maxWidth: .infinity)
                }
                .buttonStyle(.bordered)
                .disabled(model.busy || model.configuration == nil)

                Button {
                    Task { await model.refreshSelf() }
                } label: {
                    Label("Refrescar iBridge", systemImage: "bolt.horizontal.circle")
                        .frame(maxWidth: .infinity)
                }
                .buttonStyle(.bordered)
                .disabled(model.busy || model.configuration == nil)
            }
        }
        .frame(maxWidth: .infinity)
        .padding(16)
        .background(.thinMaterial, in: RoundedRectangle(cornerRadius: 18))
    }

    private var statusCard: some View {
        HStack(alignment: .top, spacing: 12) {
            if model.busy {
                ProgressView()
            } else {
                Image(systemName: "info.circle")
                    .foregroundStyle(.secondary)
            }
            Text(model.status)
                .font(.footnote)
                .frame(maxWidth: .infinity, alignment: .leading)
        }
        .padding(14)
        .background(Color.secondary.opacity(0.08), in: RoundedRectangle(cornerRadius: 14))
    }
}
