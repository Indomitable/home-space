Feature: Search, new folders and uploads on mobile
  The Android and iOS apps find files by name, create folders, and upload files the user picks. Sync
  has its own feature file, mobile-sync.feature.

  Background:
    Given alice is signed in on a phone

  # --- Search ------------------------------------------------------------------------------

  Rule: Search covers every file and folder by name, wherever it is

    Scenario: A result says where it is
      Given a file "holiday-plan.txt" in "Docs/Nested"
      When alice searches for "hol"
      Then the results show "holiday-plan.txt" in "Home / Docs / Nested"

    Scenario: Nothing matches
      When alice searches for "zzqq"
      Then the app says there are no results

    Scenario: Tapping a file opens it
      Given a file "holiday-plan.txt" in "Docs/Nested"
      When alice searches for "hol" and taps the file
      Then it opens, and stays on the phone as an offline copy

    Scenario: Tapping a folder goes there
      Given a folder "Nested" in "Docs"
      When alice searches for "Nest" and taps the folder
      Then the browser shows "Nested"
      And going back leads up the way the folder really is on Android, and back to the results on iOS

    Scenario: Showing a file in its folder
      Given a file "note.txt" in "Docs"
      When alice searches for "note" and chooses "Show in folder"
      Then the browser shows "Docs"

  # --- New folders -------------------------------------------------------------------------

  Rule: A new folder goes in the folder on screen

    Scenario: Creating a folder
      Given alice is in "Docs"
      When alice creates a folder "Fresh"
      Then "Fresh" appears in "Docs"

    Scenario: The name is taken
      Given a folder "Fresh" in Home
      When alice creates a folder "Fresh" in Home
      Then the dialog stays open and says the name exists

  # --- Uploads -----------------------------------------------------------------------------

  Rule: Picked files go up one at a time into the folder they were picked in

    Scenario: Several files, while moving around
      Given alice is in "Docs"
      When alice picks "a.txt" and "b.txt" and opens another folder
      Then both end up in "Docs"
      And until then every folder shows a row for each

    Scenario: An empty file
      When alice picks an empty file
      Then it is stored with size 0

    Scenario: A photo from the library on iOS
      When alice chooses "Upload Photos" and picks a photo
      Then the original file is stored under its own name

    Scenario: Mobile data
      Given "Sync only on Wi-Fi" is on and the phone is on mobile data
      When alice picks a file
      Then it goes up

  Rule: A name clash makes a new version, because the user chose this file and this folder

    Scenario: Different content
      Given a file "note.txt" in "Docs"
      When alice uploads a different "note.txt" to "Docs"
      Then "Docs" holds only "note.txt", with the new content as its next version

    Scenario: The same content
      Given a file "note.txt" in "Docs"
      When alice uploads the same "note.txt" to "Docs"
      Then "Docs" holds only "note.txt", at the same version

  Rule: An upload that stops can continue, and giving up leaves nothing behind

    Scenario: The server goes away
      Given an upload that is part way through
      When the server stays out of reach
      Then the row says so and offers Retry
      When the server is back and alice taps Retry
      Then the upload continues from what the server has
      And the stored file is identical to the picked one

    Scenario: Cancelling
      Given an upload that is part way through
      When alice cancels it
      Then the row goes, the server ends the upload, and the temporary copy is removed

    Scenario: Signing out
      Given an upload that is part way through
      When alice signs out
      Then the server ends the upload, and the temporary copy is removed
