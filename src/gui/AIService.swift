import Foundation
import Security

enum AIProvider: String, CaseIterable {
    case openai
    case anthropic
    case openrouter

    var displayName: String {
        switch self {
        case .openai: return "OpenAI"
        case .anthropic: return "Anthropic"
        case .openrouter: return "OpenRouter"
        }
    }
}

final class AIService {
    static let shared = AIService()

    private let servicePrefix = "com.macmon.ai"

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
        addQuery[kSecAttrAccessible as String] = kSecAttrAccessibleAfterFirstUnlock
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
                             completion: @escaping (Result<[Int], Error>) -> Void) {
        guard let apiKey = loadAPIKey(provider: provider) else {
            completion(.failure(NSError(domain: "AIService", code: 401, userInfo: [NSLocalizedDescriptionKey: "Missing API key in Keychain"])))
            return
        }

        let payload = buildRequest(provider: provider, model: model, profile: profile, processSummary: processSummary)
        let endpoint = endpointURL(provider: provider)
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
        }
        request.timeoutInterval = 30

        do {
            request.httpBody = try JSONSerialization.data(withJSONObject: payload, options: [])
        } catch {
            completion(.failure(error))
            return
        }

        URLSession.shared.dataTask(with: request) { data, _, error in
            if let error = error {
                completion(.failure(error))
                return
            }
            guard let data = data else {
                completion(.failure(NSError(domain: "AIService", code: 500, userInfo: [NSLocalizedDescriptionKey: "Empty AI response"])))
                return
            }
            do {
                let pids = try self.parseSuggestedPIDs(provider: provider, data: data)
                completion(.success(pids))
            } catch {
                completion(.failure(error))
            }
        }.resume()
    }

    private func endpointURL(provider: AIProvider) -> URL {
        switch provider {
        case .openai:
            return URL(string: "https://api.openai.com/v1/chat/completions")!
        case .anthropic:
            return URL(string: "https://api.anthropic.com/v1/messages")!
        case .openrouter:
            return URL(string: "https://openrouter.ai/api/v1/chat/completions")!
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
        3) Return only this shape: {"pids":[123,456]}
        4) Include only non critical user space PIDs.
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
        }
    }

    private func parseSuggestedPIDs(provider: AIProvider, data: Data) throws -> [Int] {
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
        }

        let extracted = extractFirstJSONObject(text)
        guard let jsonData = extracted.data(using: .utf8),
              let obj = try JSONSerialization.jsonObject(with: jsonData, options: []) as? [String: Any],
              let pids = obj["pids"] as? [Int] else {
            throw NSError(domain: "AIService", code: 422, userInfo: [NSLocalizedDescriptionKey: "AI did not return strict JSON with pids"])
        }
        return pids
    }

    private func extractFirstJSONObject(_ text: String) -> String {
        guard let start = text.firstIndex(of: "{"), let end = text.lastIndex(of: "}") else {
            return text
        }
        return String(text[start...end])
    }
}
