import Foundation

enum I18n {
    private static let tableName = "Localizable"
    private static let fallbackLanguage = "en"

    private static var cache: [String: [String: String]] = [:]
    private static let cacheLock = NSLock()

    private static var selectedLanguage: String {
        if let forced = ProcessInfo.processInfo.environment["MACMON_LANG"], !forced.isEmpty {
            return String(forced.prefix(2)).lowercased()
        }
        if let preferred = Locale.preferredLanguages.first, !preferred.isEmpty {
            return String(preferred.prefix(2)).lowercased()
        }
        return fallbackLanguage
    }

    static func tr(_ key: String) -> String {
        let lang = selectedLanguage
        if let value = value(for: key, language: lang) {
            return value
        }
        if let fallback = value(for: key, language: fallbackLanguage) {
            return fallback
        }
        return key
    }

    static func trf(_ key: String, _ args: CVarArg...) -> String {
        let template = tr(key)
        return withVaList(args) { ptr in
            NSString(format: template, arguments: ptr) as String
        }
    }

    private static func value(for key: String, language: String) -> String? {
        cacheLock.lock()
        defer { cacheLock.unlock() }
        if cache[language] == nil {
            cache[language] = loadTable(language: language)
        }
        return cache[language]?[key]
    }

    private static func loadTable(language: String) -> [String: String] {
        let fm = FileManager.default
        let home = ProcessInfo.processInfo.environment["MACMON_HOME"]
            ?? (NSHomeDirectory() + "/.local/libexec/macmon")
        let path = home + "/src/gui/Resources/\(language).lproj/\(tableName).strings"
        guard fm.fileExists(atPath: path) else { return [:] }
        guard let dict = NSDictionary(contentsOfFile: path) as? [String: String] else { return [:] }
        return dict
    }
}

func L(_ key: String) -> String {
    I18n.tr(key)
}

func LF(_ key: String, _ args: CVarArg...) -> String {
    let template = I18n.tr(key)
    return withVaList(args) { ptr in
        NSString(format: template, arguments: ptr) as String
    }
}
