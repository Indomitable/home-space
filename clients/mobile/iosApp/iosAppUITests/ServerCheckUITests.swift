import XCTest

/// Smoke test: the app reaches a dev server on localhost:8080 through the Rust client, and offers
/// the sign-in it reports. Needs the server running (`cargo run -p app -- serve` in server/), with
/// password sign-in on, and nobody signed in on the simulator.
final class ServerCheckUITests: XCTestCase {
    func testOffersSignInOnceTheServerAnswers() {
        let app = XCUIApplication()
        app.launch()

        let signIn = app.buttons["Sign in"]
        XCTAssertTrue(signIn.waitForExistence(timeout: 10), app.debugDescription)
        XCTAssertTrue(app.textFields["Username"].exists)
        XCTAssertTrue(app.secureTextFields["Password"].exists)
    }
}
