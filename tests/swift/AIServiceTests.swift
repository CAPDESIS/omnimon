import XCTest

class AIServiceExtractionTests: XCTestCase {
    func testProviderListIncludesGemini() {
        XCTAssertTrue(AIProvider.allCases.contains(.gemini))
        XCTAssertEqual(AIProvider.gemini.displayName, "Gemini")
    }

    func testExtractPIDCandidatesFromStrictJSON() {
        let text = "{\"pids\":[123,456,789]}"
        let pids = AIService.extractPIDCandidates(from: text)
        XCTAssertEqual(pids, [123, 456, 789])
    }

    func testExtractPIDCandidatesRejectsLooseText() {
        // Loose brackets without {"pids":...} wrapper must be rejected
        let text = "Here are safe candidates: [123, 456] based on your profile."
        let pids = AIService.extractPIDCandidates(from: text)
        XCTAssertTrue(pids.isEmpty)
    }

    func testExtractPIDCandidatesFromWrappedJSON() {
        let text = "Sure! {\"pids\":[42, 99]} hope that helps."
        let pids = AIService.extractPIDCandidates(from: text)
        XCTAssertEqual(pids, [42, 99])
    }

    func testSanitizeSuggestedPIDsFiltersProtectedAndUnknown() {
        let processTable: [Int: String] = [
            111: "WindowServer",
            222: "my-app",
        ]
        let result = AIService.sanitizeSuggestedPIDs([111, 222, 333], processTable: processTable)
        XCTAssertFalse(result.contains(111))
        XCTAssertFalse(result.contains(333))
    }
}
