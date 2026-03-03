import Cocoa

final class PreferencesWindowController: NSWindowController {
    private let providerPopup = NSPopUpButton(frame: .zero, pullsDown: false)
    private let modelField = NSTextField(string: "gpt-4o-mini")
    private let keyField = NSSecureTextField(string: "")
    private let statusLabel = NSTextField(labelWithString: "")

    convenience init() {
        let rect = NSRect(x: 0, y: 0, width: 480, height: 220)
        let window = NSWindow(contentRect: rect,
                              styleMask: [.titled, .closable],
                              backing: .buffered,
                              defer: false)
        self.init(window: window)
        window.title = L("prefs.window.title")
        window.center()
        setupUI()
    }

    private func setupUI() {
        guard let content = window?.contentView else { return }

        let providerLabel = NSTextField(labelWithString: L("prefs.provider"))
        let modelLabel = NSTextField(labelWithString: L("prefs.model"))
        let keyLabel = NSTextField(labelWithString: L("prefs.api_key"))
        let saveButton = NSButton(title: L("prefs.save"), target: self, action: #selector(savePreferences))

        AIProvider.allCases.forEach { providerPopup.addItem(withTitle: $0.displayName) }
        statusLabel.textColor = .secondaryLabelColor

        let fields: [NSView] = [providerLabel, providerPopup, modelLabel, modelField, keyLabel, keyField, saveButton, statusLabel]
        for view in fields {
            view.translatesAutoresizingMaskIntoConstraints = false
            content.addSubview(view)
        }

        NSLayoutConstraint.activate([
            providerLabel.topAnchor.constraint(equalTo: content.topAnchor, constant: 20),
            providerLabel.leadingAnchor.constraint(equalTo: content.leadingAnchor, constant: 20),
            providerLabel.widthAnchor.constraint(equalToConstant: 110),

            providerPopup.centerYAnchor.constraint(equalTo: providerLabel.centerYAnchor),
            providerPopup.leadingAnchor.constraint(equalTo: providerLabel.trailingAnchor, constant: 10),
            providerPopup.trailingAnchor.constraint(equalTo: content.trailingAnchor, constant: -20),

            modelLabel.topAnchor.constraint(equalTo: providerLabel.bottomAnchor, constant: 16),
            modelLabel.leadingAnchor.constraint(equalTo: content.leadingAnchor, constant: 20),
            modelLabel.widthAnchor.constraint(equalToConstant: 110),

            modelField.centerYAnchor.constraint(equalTo: modelLabel.centerYAnchor),
            modelField.leadingAnchor.constraint(equalTo: modelLabel.trailingAnchor, constant: 10),
            modelField.trailingAnchor.constraint(equalTo: content.trailingAnchor, constant: -20),

            keyLabel.topAnchor.constraint(equalTo: modelLabel.bottomAnchor, constant: 16),
            keyLabel.leadingAnchor.constraint(equalTo: content.leadingAnchor, constant: 20),
            keyLabel.widthAnchor.constraint(equalToConstant: 110),

            keyField.centerYAnchor.constraint(equalTo: keyLabel.centerYAnchor),
            keyField.leadingAnchor.constraint(equalTo: keyLabel.trailingAnchor, constant: 10),
            keyField.trailingAnchor.constraint(equalTo: content.trailingAnchor, constant: -20),

            saveButton.topAnchor.constraint(equalTo: keyLabel.bottomAnchor, constant: 20),
            saveButton.trailingAnchor.constraint(equalTo: content.trailingAnchor, constant: -20),

            statusLabel.centerYAnchor.constraint(equalTo: saveButton.centerYAnchor),
            statusLabel.leadingAnchor.constraint(equalTo: content.leadingAnchor, constant: 20),
            statusLabel.trailingAnchor.constraint(lessThanOrEqualTo: saveButton.leadingAnchor, constant: -12),
        ])

        restoreSavedValues()
    }

    private func selectedProvider() -> AIProvider {
        let title = providerPopup.titleOfSelectedItem ?? "OpenAI"
        return AIProvider.allCases.first(where: { $0.displayName == title }) ?? .openai
    }

    private func restoreSavedValues() {
        let defaults = UserDefaults.standard
        let providerRaw = defaults.string(forKey: "macmon.ai.provider") ?? AIProvider.openai.rawValue
        let model = defaults.string(forKey: "macmon.ai.model") ?? "gpt-4o-mini"
        if let provider = AIProvider(rawValue: providerRaw),
           let idx = AIProvider.allCases.firstIndex(of: provider) {
            providerPopup.selectItem(at: idx)
            keyField.stringValue = AIService.shared.loadAPIKey(provider: provider) ?? ""
        }
        modelField.stringValue = model
    }

    @objc private func savePreferences() {
        let provider = selectedProvider()
        let key = keyField.stringValue.trimmingCharacters(in: .whitespacesAndNewlines)
        let model = modelField.stringValue.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !key.isEmpty, !model.isEmpty else {
            statusLabel.stringValue = L("prefs.status.invalid")
            return
        }
        guard AIService.shared.saveAPIKey(key, provider: provider) else {
            statusLabel.stringValue = L("prefs.status.keychain_error")
            return
        }
        let defaults = UserDefaults.standard
        defaults.set(provider.rawValue, forKey: "macmon.ai.provider")
        defaults.set(model, forKey: "macmon.ai.model")
        statusLabel.stringValue = L("prefs.status.saved")
    }
}
