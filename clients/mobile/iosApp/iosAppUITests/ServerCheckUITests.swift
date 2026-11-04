import XCTest

/// Smoke test: the app reaches a dev server on localhost:8080 through the Rust client.
/// Needs the server running (`cargo run -p app -- serve` in server/).
final class ServerCheckUITests: XCTestCase {
    func testChecksServer() {
        let app = XCUIApplication()
        app.launch()
        app.buttons["Check server"].tap()

        let result = app.staticTexts["Server status: ok"]
        XCTAssertTrue(result.waitForExistence(timeout: 10), app.debugDescription)
    }
}
