import Foundation
import Security
import Darwin

struct AISuggestion {
    let pid: Int
    let reason: String
}

enum AIProvider: String, CaseIterable {
    case openai
    case anthropic
    case openrouter
    case gemini

    var displayName: String {
        switch self {
        case .openai: return "OpenAI"
        case .anthropic: return "Anthropic"
        case .openrouter: return "OpenRouter"
        case .gemini: return "Gemini"
        }
    }

    var defaultModel: String {
        switch self {
        case .openai: return "gpt-4o-mini"
        case .anthropic: return "claude-3-5-sonnet-latest"
        case .openrouter: return "openai/gpt-4o-mini"
        case .gemini: return "gemini-1.5-flash"
        }
    }
}

final class AIService {
    static let shared = AIService()

    private let servicePrefix = "com.macmon.ai"
    static let immutableProtectedProcessNames: Set<String> = [
        "WindowServer",
        "coreaudiod",
        "AudioComponentRegistrar",
        "coremediaiod",
        "VTDecoderXPCService",
        "VTEncoderXPCService",
        "kernel_task",
        "launchd",
        "syslogd",
        "logd",
        "notifyd",
        "loginwindow",
        "bluetoothd",
        "fseventsd",
        "mds",
        "opendirectoryd",
        "configd",
        "powerd",
        "thermalmonitord",
    ]

    static let maxSuggestedPIDs = 20

    private init() {}

    func saveAPIKey(_ key: String, provider: AIProvider) -> Bool {
        let account = provider.rawValue
        let data = Data(key.utf8)

        let baseQuery: [String: Any] = [
            kSecClass as String: kSecClassGenericPassword,
            kSecAttrService as String: servicePrefix,
            kSecAttrAccount as String: account,
        ]

        SecItemDelete(baseQuery as CFDictionary)

        var addQuery = baseQuery
        addQuery[kSecValueData as String] = data
        addQuery[kSecAttrAccessible as String] = kSecAttrAccessibleWhenUnlocked
        return SecItemAdd(addQuery as CFDictionary, nil) == errSecSuccess
    }

    func loadAPIKey(provider: AIProvider) -> String? {
        let query: [String: Any] = [
            kSecClass as String: kSecClassGenericPassword,
            kSecAttrService as String: servicePrefix,
            kSecAttrAccount as String: provider.rawValue,
            kSecReturnData as String: true,
            kSecMatchLimit as String: kSecMatchLimitOne,
        ]

        var item: CFTypeRef?
        let status = SecItemCopyMatching(query as CFDictionary, &item)
        guard status == errSecSuccess,
              let data = item as? Data,
              let key = String(data: data, encoding: .utf8),
              !key.isEmpty else {
            return nil
        }
        return key
    }

    func analyzeTopProcesses(provider: AIProvider,
                             model: String,
                             profile: String,
                             processSummary: [[String: Any]],
                             completion: @escaping (Result<[AISuggestion], Error>) -> Void) {
        guard let apiKey = loadAPIKey(provider: provider) else {
            completion(.failure(NSError(domain: "AIService", code: 401, userInfo: [NSLocalizedDescriptionKey: "Missing API key in Keychain"])))
            return
        }

        let payload = buildRequest(provider: provider, model: model, profile: profile, processSummary: processSummary)
        let endpoint = endpointURL(provider: provider, model: model, apiKey: apiKey)
        var request = URLRequest(url: endpoint)
        request.httpMethod = "POST"
        request.setValue("application/json", forHTTPHeaderField: "Content-Type")
        switch provider {
        case .openai:
            request.setValue("Bearer \(apiKey)", forHTTPHeaderField: "Authorization")
        case .openrouter:
            request.setValue("Bearer \(apiKey)", forHTTPHeaderField: "Authorization")
        case .anthropic:
            request.setValue(apiKey, forHTTPHeaderField: "x-api-key")
            request.setValue("2023-06-01", forHTTPHeaderField: "anthropic-version")
        case .gemini:
            request.setValue(apiKey, forHTTPHeaderField: "x-goog-api-key")
        }
        request.timeoutInterval = 30

        do {
            request.httpBody = try JSONSerialization.data(withJSONObject: payload, options: [])
        } catch {
            completion(.failure(error))
            return
        }

        URLSession.shared.dataTask(with: request) { data, response, error in
            if let error = error {
                completion(.failure(error))
                return
            }
            guard let data = data else {
                completion(.failure(NSError(domain: "AIService", code: 500, userInfo: [NSLocalizedDescriptionKey: "Empty AI response"])))
                return
            }
            // Inspect HTTP status code and surface provider error messages
            if let httpResponse = response as? HTTPURLResponse, !(200...299).contains(httpResponse.statusCode) {
                let statusCode = httpResponse.statusCode
                var message = "HTTP \(statusCode)"
                // Try to extract error message from provider JSON body
                if let json = try? JSONSerialization.jsonObject(with: data, options: []) as? [String: Any] {
                    if let err = json["error"] as? [String: Any], let msg = err["message"] as? String {
                        message = msg  // OpenAI / OpenRouter / Gemini format
                    } else if let err = json["error"] as? [String: Any], let msg = err["type"] as? String {
                        message = msg
                    } else if let msg = json["message"] as? String {
                        message = msg  // Generic format
                    }
                }
                switch statusCode {
                case 401:
                    message = "Invalid API Key: \(message)"
                case 429:
                    message = "Rate Limit Exceeded: \(message)"
                case 500...599:
                    message = "Server Error (\(statusCode)): \(message)"
                default:
                    message = "API Error (\(statusCode)): \(message)"
                }
                completion(.failure(NSError(domain: "AIService", code: statusCode, userInfo: [NSLocalizedDescriptionKey: message])))
                return
            }
            do {
                let suggestions = try self.parseSuggestions(provider: provider, data: data)
                completion(.success(suggestions))
            } catch {
                completion(.failure(error))
            }
        }.resume()
    }

    private func endpointURL(provider: AIProvider, model: String, apiKey: String) -> URL {
        switch provider {
        case .openai:
            return URL(string: "https://api.openai.com/v1/chat/completions")!
        case .anthropic:
            return URL(string: "https://api.anthropic.com/v1/messages")!
        case .openrouter:
            return URL(string: "https://openrouter.ai/api/v1/chat/completions")!
        case .gemini:
            let encodedModel = model.addingPercentEncoding(withAllowedCharacters: .urlPathAllowed) ?? model
            return URL(string: "https://generativelanguage.googleapis.com/v1beta/models/\(encodedModel):generateContent")!
        }
    }

    private func prompt(profile: String, processSummary: [[String: Any]]) -> String {
        let json = (try? JSONSerialization.data(withJSONObject: processSummary, options: [.sortedKeys])) ?? Data()
        let jsonString = String(data: json, encoding: .utf8) ?? "[]"
        return """
        You are a macOS optimization assistant. Analyze process data and return strict JSON only.
        Constraints:
        1) Never output shell commands.
        2) Never include Apple system processes or audio/video critical services.
        3) Return only this shape: {"suggestions":[{"pid":123,"reason":"short explanation"}]}
        4) Include only non critical user space PIDs.
        5) Each reason must be a brief, human-readable explanation (e.g. "high RAM usage while idle").
        Current profile: \(profile)
        Processes: \(jsonString)
        """
    }

    private func buildRequest(provider: AIProvider,
                              model: String,
                              profile: String,
                              processSummary: [[String: Any]]) -> [String: Any] {
        let msg = prompt(profile: profile, processSummary: processSummary)
        switch provider {
        case .openai, .openrouter:
            return [
                "model": model,
                "temperature": 0,
                "messages": [
                    ["role": "system", "content": "Return strict JSON only."],
                    ["role": "user", "content": msg],
                ],
            ]
        case .anthropic:
            return [
                "model": model,
                "max_tokens": 512,
                "temperature": 0,
                "messages": [
                    ["role": "user", "content": msg],
                ],
            ]
        case .gemini:
            return [
                "generationConfig": [
                    "temperature": 0,
                    "maxOutputTokens": 512,
                ],
                "contents": [
                    [
                        "role": "user",
                        "parts": [["text": msg]],
                    ],
                ],
            ]
        }
    }

    private func parseSuggestions(provider: AIProvider, data: Data) throws -> [AISuggestion] {
        let root = try JSONSerialization.jsonObject(with: data, options: [])
        let text: String
        switch provider {
        case .openai, .openrouter:
            guard let dict = root as? [String: Any],
                  let choices = dict["choices"] as? [[String: Any]],
                  let first = choices.first,
                  let message = first["message"] as? [String: Any],
                  let content = message["content"] as? String else {
                throw NSError(domain: "AIService", code: 422, userInfo: [NSLocalizedDescriptionKey: "Invalid AI response shape"])
            }
            text = content
        case .anthropic:
            guard let dict = root as? [String: Any],
                  let content = dict["content"] as? [[String: Any]],
                  let first = content.first,
                  let value = first["text"] as? String else {
                throw NSError(domain: "AIService", code: 422, userInfo: [NSLocalizedDescriptionKey: "Invalid AI response shape"])
            }
            text = value
        case .gemini:
            guard let dict = root as? [String: Any],
                  let candidates = dict["candidates"] as? [[String: Any]],
                  let first = candidates.first,
                  let content = first["content"] as? [String: Any],
                  let parts = content["parts"] as? [[String: Any]],
                  let part = parts.first,
                  let value = part["text"] as? String else {
                throw NSError(domain: "AIService", code: 422, userInfo: [NSLocalizedDescriptionKey: "Invalid AI response shape"])
            }
            text = value
        }

        let suggestions = AIService.extractSuggestions(from: text)
        guard !suggestions.isEmpty else {
            throw NSError(domain: "AIService", code: 422, userInfo: [NSLocalizedDescriptionKey: "AI did not return valid PID candidates"])
        }
        var seen = Set<Int>()
        var unique: [AISuggestion] = []
        for s in suggestions {
            if !seen.contains(s.pid) {
                seen.insert(s.pid)
                unique.append(s)
            }
        }
        return Array(unique.prefix(AIService.maxSuggestedPIDs))
    }

    static func extractSuggestions(from text: String) -> [AISuggestion] {
        let normalized = text.trimmingCharacters(in: .whitespacesAndNewlines)

        guard let start = normalized.firstIndex(of: "{"),
              let end = normalized.lastIndex(of: "}") else { return [] }
        let jsonFragment = String(normalized[start...end])
        guard let data = jsonFragment.data(using: .utf8),
              let obj = try? JSONSerialization.jsonObject(with: data, options: []) as? [String: Any] else { return [] }

        // New format: {"suggestions":[{"pid":123,"reason":"..."}]}
        if let arr = obj["suggestions"] as? [[String: Any]], !arr.isEmpty {
            return arr.compactMap { entry in
                guard let pid = entry["pid"] as? Int, pid > 1 else { return nil }
                let reason = entry["reason"] as? String ?? L("picker.ai.reason.generic")
                return AISuggestion(pid: pid, reason: reason)
            }
        }

        // Backward compat: {"pids":[123,456]}
        if let pids = obj["pids"] as? [Int], !pids.isEmpty {
            return pids.filter { $0 > 1 }.map { AISuggestion(pid: $0, reason: L("picker.ai.reason.generic")) }
        }

        return []
    }

    static func sanitizeSuggestions(_ suggestions: [AISuggestion], processTable: [Int: String]) -> [AISuggestion] {
        var filtered: [AISuggestion] = []
        var seen = Set<Int>()
        for s in suggestions where s.pid > 1 {
            guard let name = processTable[s.pid], !name.isEmpty else { continue }
            guard !immutableProtectedProcessNames.contains(name) else { continue }
            if kill(pid_t(s.pid), 0) == 0 && !seen.contains(s.pid) {
                seen.insert(s.pid)
                filtered.append(s)
            }
        }
        return filtered
    }
}
