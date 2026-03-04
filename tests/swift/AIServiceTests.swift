import XCTest

class AIServiceExtractionTests: XCTestCase {
    func testProviderListIncludesGemini() {
        XCTAssertTrue(AIProvider.allCases.contains(.gemini))
        XCTAssertEqual(AIProvider.gemini.displayName, "Gemini")
    }

    // v3.0: extractSuggestions replaces extractPIDCandidates

    func testExtractSuggestionsFromNewFormat() {
        let text = "{\"suggestions\":[{\"pid\":123,\"reason\":\"high RAM\"},{\"pid\":456,\"reason\":\"idle\"}]}"
        let suggestions = AIService.extractSuggestions(from: text)
        XCTAssertEqual(suggestions.count, 2)
        XCTAssertEqual(suggestions[0].pid, 123)
        XCTAssertEqual(suggestions[0].reason, "high RAM")
        XCTAssertEqual(suggestions[1].pid, 456)
        XCTAssertEqual(suggestions[1].reason, "idle")
    }

    func testExtractSuggestionsBackwardCompatWithPidsFormat() {
        let text = "{\"pids\":[123,456,789]}"
        let suggestions = AIService.extractSuggestions(from: text)
        XCTAssertEqual(suggestions.count, 3)
        XCTAssertEqual(suggestions[0].pid, 123)
        XCTAssertEqual(suggestions[1].pid, 456)
        XCTAssertEqual(suggestions[2].pid, 789)
    }

    func testExtractSuggestionsRejectsLooseText() {
        let text = "Here are safe candidates: [123, 456] based on your profile."
        let suggestions = AIService.extractSuggestions(from: text)
        XCTAssertTrue(suggestions.isEmpty)
    }

    func testExtractSuggestionsFromWrappedJSON() {
        let text = "Sure! {\"suggestions\":[{\"pid\":42,\"reason\":\"test\"}]} hope that helps."
        let suggestions = AIService.extractSuggestions(from: text)
        XCTAssertEqual(suggestions.count, 1)
        XCTAssertEqual(suggestions[0].pid, 42)
        XCTAssertEqual(suggestions[0].reason, "test")
    }

    func testExtractSuggestionsBackwardCompatWrapped() {
        let text = "Sure! {\"pids\":[42, 99]} hope that helps."
        let suggestions = AIService.extractSuggestions(from: text)
        XCTAssertEqual(suggestions.count, 2)
        XCTAssertEqual(suggestions[0].pid, 42)
        XCTAssertEqual(suggestions[1].pid, 99)
    }

    func testSanitizeSuggestionsFiltersProtectedAndUnknown() {
        let processTable: [Int: String] = [
            111: "WindowServer",
            222: "my-app",
        ]
        let input = [
            AISuggestion(pid: 111, reason: "test"),
            AISuggestion(pid: 222, reason: "high RAM"),
            AISuggestion(pid: 333, reason: "idle"),
        ]
        let result = AIService.sanitizeSuggestions(input, processTable: processTable)
        // 111 is protected (WindowServer), 333 is not in processTable
        XCTAssertFalse(result.contains(where: { $0.pid == 111 }))
        XCTAssertFalse(result.contains(where: { $0.pid == 333 }))
    }

    func testExtractSuggestionsFiltersPID1() {
        let text = "{\"suggestions\":[{\"pid\":1,\"reason\":\"launchd\"},{\"pid\":500,\"reason\":\"safe\"}]}"
        let suggestions = AIService.extractSuggestions(from: text)
        XCTAssertEqual(suggestions.count, 1)
        XCTAssertEqual(suggestions[0].pid, 500)
    }
}
