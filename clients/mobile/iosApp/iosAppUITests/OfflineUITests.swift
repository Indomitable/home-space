import XCTest

/// Offline copies against a dev server on localhost:8080: opens a file, and makes a folder available
/// offline. Needs a user on the server, from TEST_RUNNER_HOMESPACE_E2E_USER and
/// TEST_RUNNER_HOMESPACE_E2E_PASSWORD, with an image `real.png` and a folder `Docs` in Home. With
/// TEST_RUNNER_HOMESPACE_E2E_HOLD_SECONDS it then stays open that long, to change things from outside.
final class OfflineUITests: XCTestCase {
    func testOpeningAndKeepingOffline() throws {
        let environment = ProcessInfo.processInfo.environment
        guard let user = environment["HOMESPACE_E2E_USER"],
            let password = environment["HOMESPACE_E2E_PASSWORD"]
        else {
            throw XCTSkip("no test user given")
        }
        let app = XCUIApplication()
        app.launch()
        let menu = app.buttons["Menu"]
        if !menu.waitForExistence(timeout: 5) {
            let signIn = app.buttons["Sign in"]
            XCTAssertTrue(signIn.waitForExistence(timeout: 15), app.debugDescription)
            app.textFields["Username"].tap()
            app.textFields["Username"].typeText(user)
            app.secureTextFields["Password"].tap()
            app.secureTextFields["Password"].typeText(password)
            signIn.tap()
            XCTAssertTrue(menu.waitForExistence(timeout: 15), app.debugDescription)
        }

        // Opening a file shows Quick Look with its markup tool, and leaves a copy.
        app.staticTexts["real.png"].tap()
        let preview = app.otherElements["QLPreviewControllerView"]
        XCTAssertTrue(preview.waitForExistence(timeout: 15), app.debugDescription)
        // Quick Look starts full screen, and a tap brings its controls.
        preview.tap()
        let close = app.buttons["close"]
        XCTAssertTrue(close.waitForExistence(timeout: 5), app.debugDescription)
        print("QUICKLOOK BUTTONS:", app.buttons.allElementsBoundByIndex.map(\.label))
        close.tap()
        let kept = app.images["Available offline"]
        XCTAssertTrue(kept.waitForExistence(timeout: 15), app.debugDescription)

        app.staticTexts["Docs"].press(forDuration: 1.2)
        app.buttons["Make Available Offline"].tap()

        let hold = environment["HOMESPACE_E2E_HOLD_SECONDS"].flatMap(Double.init) ?? 0
        if hold > 0 {
            Thread.sleep(forTimeInterval: hold)
        }
        print("BADGES:", app.images.allElementsBoundByIndex.map(\.label))
        add(XCTAttachment(screenshot: app.screenshot()))
    }
}
