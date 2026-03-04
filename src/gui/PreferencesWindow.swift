import Cocoa

final class PreferencesWindowController: NSWindowController {
    private let providerPopup = NSPopUpButton(frame: .zero, pullsDown: false)
    private let modelField = NSTextField(string: "gpt-4o-mini")
    private let keySecureField = NSSecureTextField(string: "")
    private let keyPlainField = NSTextField(string: "")
    private let statusLabel = NSTextField(labelWithString: "")

    // Rules tab fields
    private var rulesFields: [String: NSTextField] = [:]
    private var rulesDiskIOCheckbox: NSButton?
    private var privacyURLCheckbox: NSButton?
    private let rulesStatusLabel = NSTextField(labelWithString: "")

    convenience init() {
        let rect = NSRect(x: 0, y: 0, width: 520, height: 480)
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

        let tabView = NSTabView(frame: .zero)
        tabView.translatesAutoresizingMaskIntoConstraints = false
        content.addSubview(tabView)

        NSLayoutConstraint.activate([
            tabView.topAnchor.constraint(equalTo: content.topAnchor, constant: 8),
            tabView.leadingAnchor.constraint(equalTo: content.leadingAnchor, constant: 8),
            tabView.trailingAnchor.constraint(equalTo: content.trailingAnchor, constant: -8),
            tabView.bottomAnchor.constraint(equalTo: content.bottomAnchor, constant: -8),
        ])

        // --- AI Settings Tab ---
        let aiTab = NSTabViewItem(identifier: "ai_settings")
        aiTab.label = L("prefs.tab.ai_settings")
        let aiView = NSView(frame: .zero)
        aiTab.view = aiView
        setupAISettingsTab(in: aiView)
        tabView.addTabViewItem(aiTab)

        // --- Rules Tab ---
        let rulesTab = NSTabViewItem(identifier: "rules")
        rulesTab.label = L("prefs.tab.rules")
        let rulesView = NSView(frame: .zero)
        rulesTab.view = rulesView
        setupRulesTab(in: rulesView)
        tabView.addTabViewItem(rulesTab)

        restoreSavedValues()
        loadRulesFromConfig()
    }

    // MARK: - AI Settings Tab

    private func setupAISettingsTab(in container: NSView) {
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
        keyPlainField.isHidden = true
        pasteModelButton.bezelStyle = .rounded
        pasteKeyButton.bezelStyle = .rounded
        revealKeyButton.setButtonType(.switch)

        let fields: [NSView] = [providerLabel, providerPopup, modelLabel, modelField, keyLabel, keySecureField, keyPlainField, pasteModelButton, pasteKeyButton, revealKeyButton, securityNote, saveButton, statusLabel]
        for view in fields {
            view.translatesAutoresizingMaskIntoConstraints = false
            container.addSubview(view)
        }

        NSLayoutConstraint.activate([
            providerLabel.topAnchor.constraint(equalTo: container.topAnchor, constant: 20),
            providerLabel.leadingAnchor.constraint(equalTo: container.leadingAnchor, constant: 20),
            providerLabel.widthAnchor.constraint(equalToConstant: 110),

            providerPopup.centerYAnchor.constraint(equalTo: providerLabel.centerYAnchor),
            providerPopup.leadingAnchor.constraint(equalTo: providerLabel.trailingAnchor, constant: 10),
            providerPopup.trailingAnchor.constraint(equalTo: container.trailingAnchor, constant: -20),

            modelLabel.topAnchor.constraint(equalTo: providerLabel.bottomAnchor, constant: 16),
            modelLabel.leadingAnchor.constraint(equalTo: container.leadingAnchor, constant: 20),
            modelLabel.widthAnchor.constraint(equalToConstant: 110),

            modelField.centerYAnchor.constraint(equalTo: modelLabel.centerYAnchor),
            modelField.leadingAnchor.constraint(equalTo: modelLabel.trailingAnchor, constant: 10),
            modelField.trailingAnchor.constraint(equalTo: pasteModelButton.leadingAnchor, constant: -8),

            pasteModelButton.centerYAnchor.constraint(equalTo: modelField.centerYAnchor),
            pasteModelButton.trailingAnchor.constraint(equalTo: container.trailingAnchor, constant: -20),
            pasteModelButton.widthAnchor.constraint(equalToConstant: 72),

            keyLabel.topAnchor.constraint(equalTo: modelLabel.bottomAnchor, constant: 16),
            keyLabel.leadingAnchor.constraint(equalTo: container.leadingAnchor, constant: 20),
            keyLabel.widthAnchor.constraint(equalToConstant: 110),

            keySecureField.centerYAnchor.constraint(equalTo: keyLabel.centerYAnchor),
            keySecureField.leadingAnchor.constraint(equalTo: keyLabel.trailingAnchor, constant: 10),
            keySecureField.trailingAnchor.constraint(equalTo: pasteKeyButton.leadingAnchor, constant: -8),

            keyPlainField.centerYAnchor.constraint(equalTo: keyLabel.centerYAnchor),
            keyPlainField.leadingAnchor.constraint(equalTo: keyLabel.trailingAnchor, constant: 10),
            keyPlainField.trailingAnchor.constraint(equalTo: pasteKeyButton.leadingAnchor, constant: -8),

            pasteKeyButton.centerYAnchor.constraint(equalTo: keySecureField.centerYAnchor),
            pasteKeyButton.trailingAnchor.constraint(equalTo: container.trailingAnchor, constant: -20),
            pasteKeyButton.widthAnchor.constraint(equalToConstant: 72),

            revealKeyButton.topAnchor.constraint(equalTo: keySecureField.bottomAnchor, constant: 6),
            revealKeyButton.leadingAnchor.constraint(equalTo: keySecureField.leadingAnchor),

            securityNote.topAnchor.constraint(equalTo: revealKeyButton.bottomAnchor, constant: 8),
            securityNote.leadingAnchor.constraint(equalTo: container.leadingAnchor, constant: 20),
            securityNote.trailingAnchor.constraint(equalTo: container.trailingAnchor, constant: -20),

            saveButton.topAnchor.constraint(equalTo: securityNote.bottomAnchor, constant: 14),
            saveButton.trailingAnchor.constraint(equalTo: container.trailingAnchor, constant: -20),

            statusLabel.centerYAnchor.constraint(equalTo: saveButton.centerYAnchor),
            statusLabel.leadingAnchor.constraint(equalTo: container.leadingAnchor, constant: 20),
            statusLabel.trailingAnchor.constraint(lessThanOrEqualTo: saveButton.leadingAnchor, constant: -12),
        ])
    }

    // MARK: - Rules Tab

    private func setupRulesTab(in container: NSView) {
        let helper = NSTextField(wrappingLabelWithString: L("config.editor.helper"))
        helper.textColor = .secondaryLabelColor
        helper.font = NSFont.systemFont(ofSize: 11)
        helper.translatesAutoresizingMaskIntoConstraints = false
        container.addSubview(helper)

        let fieldDefs: [(key: String, placeholder: String)] = [
            ("ram", L("config.field.ram_free")),
            ("swap", L("config.field.swap_used")),
            ("minram", L("config.field.min_ram")),
            ("idlecpu", L("config.field.idle_cpu")),
            ("check", L("config.field.check")),
            ("idlecheck", L("config.field.idle_check")),
            ("cooldown", L("config.field.cooldown")),
            ("grace", L("config.field.kill_grace")),
        ]

        for def in fieldDefs {
            let label = NSTextField(labelWithString: def.placeholder)
            label.translatesAutoresizingMaskIntoConstraints = false
            label.font = NSFont.systemFont(ofSize: 11)
            label.textColor = .secondaryLabelColor
            container.addSubview(label)

            let field = NSTextField(string: "")
            field.translatesAutoresizingMaskIntoConstraints = false
            field.placeholderString = def.placeholder
            if let cell = field.cell as? NSTextFieldCell {
                cell.usesSingleLineMode = true
                cell.lineBreakMode = .byTruncatingTail
            }
            container.addSubview(field)
            rulesFields[def.key] = field
        }

        let diskIO = NSButton(checkboxWithTitle: L("config.field.disk_io"), target: nil, action: nil)
        diskIO.translatesAutoresizingMaskIntoConstraints = false
        container.addSubview(diskIO)
        rulesDiskIOCheckbox = diskIO

        let privacyURL = NSButton(checkboxWithTitle: L("prefs.privacy.allow_urls"), target: self, action: #selector(privacyURLToggled))
        privacyURL.translatesAutoresizingMaskIntoConstraints = false
        privacyURL.state = UserDefaults.standard.bool(forKey: "macmon.privacy.allowBrowserURLs") ? .on : .off
        container.addSubview(privacyURL)
        privacyURLCheckbox = privacyURL

        let privacyNote = NSTextField(wrappingLabelWithString: L("prefs.privacy.url_note"))
        privacyNote.textColor = .secondaryLabelColor
        privacyNote.font = NSFont.systemFont(ofSize: 10)
        privacyNote.translatesAutoresizingMaskIntoConstraints = false
        container.addSubview(privacyNote)

        let saveRulesBtn = NSButton(title: L("prefs.rules.save"), target: self, action: #selector(saveRules))
        saveRulesBtn.translatesAutoresizingMaskIntoConstraints = false
        container.addSubview(saveRulesBtn)

        rulesStatusLabel.textColor = .secondaryLabelColor
        rulesStatusLabel.translatesAutoresizingMaskIntoConstraints = false
        container.addSubview(rulesStatusLabel)

        // Layout
        var constraints: [NSLayoutConstraint] = [
            helper.topAnchor.constraint(equalTo: container.topAnchor, constant: 16),
            helper.leadingAnchor.constraint(equalTo: container.leadingAnchor, constant: 20),
            helper.trailingAnchor.constraint(equalTo: container.trailingAnchor, constant: -20),
        ]

        var anchor = helper.bottomAnchor
        let fieldKeys = fieldDefs.map { $0.key }
        for i in stride(from: 0, to: fieldKeys.count, by: 2) {
            let key1 = fieldKeys[i]
            let key2 = i + 1 < fieldKeys.count ? fieldKeys[i + 1] : nil

            if let f1 = rulesFields[key1] {
                let l1 = container.subviews.first(where: { ($0 as? NSTextField)?.stringValue == fieldDefs[i].placeholder && $0 !== f1 })
                if let l1 = l1 {
                    constraints.append(contentsOf: [
                        l1.topAnchor.constraint(equalTo: anchor, constant: 12),
                        l1.leadingAnchor.constraint(equalTo: container.leadingAnchor, constant: 20),
                        l1.widthAnchor.constraint(equalToConstant: 100),
                        f1.centerYAnchor.constraint(equalTo: l1.centerYAnchor),
                        f1.leadingAnchor.constraint(equalTo: l1.trailingAnchor, constant: 6),
                        f1.widthAnchor.constraint(equalToConstant: 80),
                    ])
                    anchor = l1.bottomAnchor
                }
            }

            if let key2 = key2, let f2 = rulesFields[key2] {
                let l2 = container.subviews.first(where: { ($0 as? NSTextField)?.stringValue == fieldDefs[i + 1].placeholder && $0 !== f2 })
                if let l2 = l2, let f1 = rulesFields[key1] {
                    constraints.append(contentsOf: [
                        l2.centerYAnchor.constraint(equalTo: f1.centerYAnchor),
                        l2.leadingAnchor.constraint(equalTo: f1.trailingAnchor, constant: 20),
                        l2.widthAnchor.constraint(equalToConstant: 100),
                        f2.centerYAnchor.constraint(equalTo: l2.centerYAnchor),
                        f2.leadingAnchor.constraint(equalTo: l2.trailingAnchor, constant: 6),
                        f2.widthAnchor.constraint(equalToConstant: 80),
                    ])
                }
            }
        }

        constraints.append(contentsOf: [
            diskIO.topAnchor.constraint(equalTo: anchor, constant: 16),
            diskIO.leadingAnchor.constraint(equalTo: container.leadingAnchor, constant: 20),

            privacyURL.topAnchor.constraint(equalTo: diskIO.bottomAnchor, constant: 12),
            privacyURL.leadingAnchor.constraint(equalTo: container.leadingAnchor, constant: 20),

            privacyNote.topAnchor.constraint(equalTo: privacyURL.bottomAnchor, constant: 2),
            privacyNote.leadingAnchor.constraint(equalTo: container.leadingAnchor, constant: 38),
            privacyNote.trailingAnchor.constraint(equalTo: container.trailingAnchor, constant: -20),

            saveRulesBtn.topAnchor.constraint(equalTo: privacyNote.bottomAnchor, constant: 12),
            saveRulesBtn.trailingAnchor.constraint(equalTo: container.trailingAnchor, constant: -20),

            rulesStatusLabel.centerYAnchor.constraint(equalTo: saveRulesBtn.centerYAnchor),
            rulesStatusLabel.leadingAnchor.constraint(equalTo: container.leadingAnchor, constant: 20),
            rulesStatusLabel.trailingAnchor.constraint(lessThanOrEqualTo: saveRulesBtn.leadingAnchor, constant: -12),
        ])

        NSLayoutConstraint.activate(constraints)
    }

    // MARK: - AI Settings Actions

    @objc private func providerChanged() {
        let provider = selectedProvider()
        setKeyValue(AIService.shared.loadAPIKey(provider: provider) ?? "")
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
            setKeyValue(text.trimmingCharacters(in: .whitespacesAndNewlines))
        }
    }

    @objc private func toggleKeyVisibility(_ sender: NSButton) {
        let show = (sender.state == .on)
        keyPlainField.isHidden = !show
        keySecureField.isHidden = show
        if show {
            keyPlainField.stringValue = keySecureField.stringValue
        } else {
            keySecureField.stringValue = keyPlainField.stringValue
        }
    }

    private func currentKeyValue() -> String {
        return keyPlainField.isHidden ? keySecureField.stringValue : keyPlainField.stringValue
    }

    private func setKeyValue(_ value: String) {
        keySecureField.stringValue = value
        keyPlainField.stringValue = value
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
            setKeyValue(AIService.shared.loadAPIKey(provider: provider) ?? "")
        }
        modelField.stringValue = model
    }

    @objc private func savePreferences() {
        let provider = selectedProvider()
        let key = currentKeyValue().trimmingCharacters(in: .whitespacesAndNewlines)
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

    // MARK: - Privacy Actions

    @objc private func privacyURLToggled(_ sender: NSButton) {
        UserDefaults.standard.set(sender.state == .on, forKey: "macmon.privacy.allowBrowserURLs")
    }

    // MARK: - Rules Actions

    private func configFilePath() -> String {
        let configDir = NSString(string: "~/.config/macmon").expandingTildeInPath
        return (configDir as NSString).appendingPathComponent("macmon.yaml")
    }

    private func loadRulesFromConfig() {
        let path = configFilePath()
        guard let yaml = try? String(contentsOfFile: path, encoding: .utf8) else { return }
        let s = ConfigQuickSettings.parse(from: yaml)
        rulesFields["ram"]?.stringValue = String(s.ramFreePercent)
        rulesFields["swap"]?.stringValue = String(s.swapUsedMB)
        rulesFields["minram"]?.stringValue = String(s.processMinRAMKB)
        rulesFields["idlecpu"]?.stringValue = String(format: "%.2f", s.idleCPUPercent)
        rulesFields["check"]?.stringValue = String(s.checkIntervalSec)
        rulesFields["idlecheck"]?.stringValue = String(s.idleCheckSec)
        rulesFields["cooldown"]?.stringValue = String(s.cooldownSec)
        rulesFields["grace"]?.stringValue = String(s.killGraceSec)
        rulesDiskIOCheckbox?.state = s.collectDiskIO ? .on : .off
    }

    @objc private func saveRules() {
        var s = ConfigQuickSettings()
        s.ramFreePercent = Int(rulesFields["ram"]?.stringValue ?? "") ?? s.ramFreePercent
        s.swapUsedMB = Int(rulesFields["swap"]?.stringValue ?? "") ?? s.swapUsedMB
        s.processMinRAMKB = Int(rulesFields["minram"]?.stringValue ?? "") ?? s.processMinRAMKB
        s.idleCPUPercent = Double(rulesFields["idlecpu"]?.stringValue ?? "") ?? s.idleCPUPercent
        s.checkIntervalSec = Int(rulesFields["check"]?.stringValue ?? "") ?? s.checkIntervalSec
        s.idleCheckSec = Int(rulesFields["idlecheck"]?.stringValue ?? "") ?? s.idleCheckSec
        s.cooldownSec = Int(rulesFields["cooldown"]?.stringValue ?? "") ?? s.cooldownSec
        s.killGraceSec = Int(rulesFields["grace"]?.stringValue ?? "") ?? s.killGraceSec
        s.collectDiskIO = rulesDiskIOCheckbox?.state == .on

        let yaml = s.renderYAML()
        let path = configFilePath()
        do {
            try yaml.write(toFile: path, atomically: true, encoding: .utf8)
            rulesStatusLabel.stringValue = L("config.editor.saved")
        } catch {
            rulesStatusLabel.stringValue = L("config.editor.error")
        }
    }
}
