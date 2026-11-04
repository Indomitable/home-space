import XCTest

/// Spike smoke test: the Rust tab reaches a dev server on localhost:8080 through UniFFI.
/// Needs the server running (`cargo run -p app -- serve` in server/).
final class RustCoreUITests: XCTestCase {
    func testRustClientChecksServer() {
        let app = XCUIApplication()
        app.launch()
        app.tabBars.buttons["Rust"].tap()
        app.buttons["Check server"].tap()

        let result = app.staticTexts.containing(
            NSPredicate(format: "label BEGINSWITH 'Server status: ok'")
        ).firstMatch
        XCTAssertTrue(result.waitForExistence(timeout: 10), app.debugDescription)
    }

    func testKotlinClientChecksServer() {
        let app = XCUIApplication()
        app.launch()
        app.tabBars.buttons["Kotlin"].tap()
        app.buttons["Check server"].tap()

        let result = app.staticTexts["Server status: ok"]
        XCTAssertTrue(result.waitForExistence(timeout: 10), app.debugDescription)
    }
}
