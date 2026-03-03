import XCTest

class AIServiceExtractionTests: XCTestCase {
    func testExtractPIDCandidatesFromStrictJSON() {
        let text = "{\"pids\":[123,456,789]}"
        let pids = AIService.extractPIDCandidates(from: text)
        XCTAssertEqual(pids, [123, 456, 789])
    }

    func testExtractPIDCandidatesFromConversationalText() {
        let text = "Here are safe candidates: [123, 456] based on your profile."
        let pids = AIService.extractPIDCandidates(from: text)
        XCTAssertEqual(pids, [123, 456])
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
