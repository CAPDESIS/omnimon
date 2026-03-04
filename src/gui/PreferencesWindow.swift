import Cocoa

final class PreferencesWindowController: NSWindowController {
    private let providerPopup = NSPopUpButton(frame: .zero, pullsDown: false)
    private let modelField = NSTextField(string: "gpt-4o-mini")
    private var keyField = NSSecureTextField(string: "")
    private let statusLabel = NSTextField(labelWithString: "")

    convenience init() {
        let rect = NSRect(x: 0, y: 0, width: 520, height: 260)
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
        let pasteModelButton = NSButton(title: L("prefs.paste"), target: self, action: #selector(pasteModel))
        let pasteKeyButton = NSButton(title: L("prefs.paste"), target: self, action: #selector(pasteKey))
        let revealKeyButton = NSButton(checkboxWithTitle: L("prefs.show_key"), target: self, action: #selector(toggleKeyVisibility))
        let securityNote = NSTextField(wrappingLabelWithString: L("prefs.security.note"))

        AIProvider.allCases.forEach { providerPopup.addItem(withTitle: $0.displayName) }
        providerPopup.target = self
        providerPopup.action = #selector(providerChanged)
        statusLabel.textColor = .secondaryLabelColor
        securityNote.textColor = .secondaryLabelColor
        securityNote.font = NSFont.systemFont(ofSize: 11)
        securityNote.maximumNumberOfLines = 2
        pasteModelButton.bezelStyle = .rounded
        pasteKeyButton.bezelStyle = .rounded
        revealKeyButton.setButtonType(.switch)

        let fields: [NSView] = [providerLabel, providerPopup, modelLabel, modelField, keyLabel, keyField, pasteModelButton, pasteKeyButton, revealKeyButton, securityNote, saveButton, statusLabel]
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
            modelField.trailingAnchor.constraint(equalTo: pasteModelButton.leadingAnchor, constant: -8),

            pasteModelButton.centerYAnchor.constraint(equalTo: modelField.centerYAnchor),
            pasteModelButton.trailingAnchor.constraint(equalTo: content.trailingAnchor, constant: -20),
            pasteModelButton.widthAnchor.constraint(equalToConstant: 72),

            keyLabel.topAnchor.constraint(equalTo: modelLabel.bottomAnchor, constant: 16),
            keyLabel.leadingAnchor.constraint(equalTo: content.leadingAnchor, constant: 20),
            keyLabel.widthAnchor.constraint(equalToConstant: 110),

            keyField.centerYAnchor.constraint(equalTo: keyLabel.centerYAnchor),
            keyField.leadingAnchor.constraint(equalTo: keyLabel.trailingAnchor, constant: 10),
            keyField.trailingAnchor.constraint(equalTo: pasteKeyButton.leadingAnchor, constant: -8),

            pasteKeyButton.centerYAnchor.constraint(equalTo: keyField.centerYAnchor),
            pasteKeyButton.trailingAnchor.constraint(equalTo: content.trailingAnchor, constant: -20),
            pasteKeyButton.widthAnchor.constraint(equalToConstant: 72),

            revealKeyButton.topAnchor.constraint(equalTo: keyField.bottomAnchor, constant: 6),
            revealKeyButton.leadingAnchor.constraint(equalTo: keyField.leadingAnchor),

            securityNote.topAnchor.constraint(equalTo: revealKeyButton.bottomAnchor, constant: 8),
            securityNote.leadingAnchor.constraint(equalTo: content.leadingAnchor, constant: 20),
            securityNote.trailingAnchor.constraint(equalTo: content.trailingAnchor, constant: -20),

            saveButton.topAnchor.constraint(equalTo: securityNote.bottomAnchor, constant: 14),
            saveButton.trailingAnchor.constraint(equalTo: content.trailingAnchor, constant: -20),

            statusLabel.centerYAnchor.constraint(equalTo: saveButton.centerYAnchor),
            statusLabel.leadingAnchor.constraint(equalTo: content.leadingAnchor, constant: 20),
            statusLabel.trailingAnchor.constraint(lessThanOrEqualTo: saveButton.leadingAnchor, constant: -12),
        ])

        restoreSavedValues()
    }

    @objc private func providerChanged() {
        let provider = selectedProvider()
        keyField.stringValue = AIService.shared.loadAPIKey(provider: provider) ?? ""
        let current = modelField.stringValue.trimmingCharacters(in: .whitespacesAndNewlines)
        if current.isEmpty || current == AIProvider.openai.defaultModel || current == AIProvider.anthropic.defaultModel || current == AIProvider.openrouter.defaultModel || current == AIProvider.gemini.defaultModel {
            modelField.stringValue = provider.defaultModel
        }
    }

    @objc private func pasteModel() {
        if let text = NSPasteboard.general.string(forType: .string) {
            modelField.stringValue = text.trimmingCharacters(in: .whitespacesAndNewlines)
        }
    }

    @objc private func pasteKey() {
        if let text = NSPasteboard.general.string(forType: .string) {
            keyField.stringValue = text.trimmingCharacters(in: .whitespacesAndNewlines)
        }
    }

    @objc private func toggleKeyVisibility(_ sender: NSButton) {
        let show = (sender.state == .on)
        if let cell = keyField.cell as? NSSecureTextFieldCell {
            cell.echosBullets = !show
        }
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
