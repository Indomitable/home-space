Feature: Mobile data sync
  The mobile apps upload marked local folders to the server (auto-upload, one way) and keep files
  that are opened or made available offline in sync (offline, two way). Auto-upload never lets a
  change on the server touch the local folder.

  Background:
    Given a user "alice" signed in on the device "Pixel"

  # --- Auto-upload ----------------------------------------------------------------------

  Rule: New and changed files go up, and the server never changes the local folder

    Background:
      Given the local folder "Camera" uploads to the server folder "Photos"

    Scenario: A new photo is uploaded
      When a photo "IMG_1.jpg" appears in "Camera"
      Then "Photos/IMG_1.jpg" exists on the server
      And its source device is "Pixel"

    Scenario: Moving an uploaded photo on the server leaves the local file alone
      Given "IMG_1.jpg" was uploaded
      When it is moved to "Archive" on the server
      Then the local file is unchanged
      And nothing is uploaded

    Scenario: Editing an uploaded photo makes a new version
      Given "IMG_1.jpg" was uploaded
      When the local file is edited
      Then "Photos/IMG_1.jpg" has a new version

    Scenario Outline: Editing a photo that left the folder on the server makes a new file
      Given "IMG_1.jpg" was uploaded
      And it was <change> on the server
      When the local file is edited
      Then a new file "Photos/IMG_1.jpg" exists on the server

      Examples:
        | change            |
        | moved to Archive  |
        | deleted           |

    Scenario: A local delete stays local
      Given "IMG_1.jpg" was uploaded
      When the local file is deleted
      Then "Photos/IMG_1.jpg" stays on the server

    Scenario: A file being written waits
      When a file in "Camera" changed a moment ago
      Then it is left for a later pass

  Rule: A file that was never uploaded does not become a version of a different file

    Scenario: Different content with the same name keeps both
      Given the server folder "Photos" has a file "IMG_1.jpg" from another phone
      When a different "IMG_1.jpg" appears in "Camera"
      Then the server folder holds "IMG_1.jpg" and "IMG_1 (1).jpg"
      And later edits of the local file go to "IMG_1 (1).jpg"

    Scenario: The same file arriving again changes nothing
      Given the answer to the last chunk of "IMG_1.jpg" was lost
      When it is uploaded again
      Then the server folder still holds one copy

  Rule: The source device says where a file came from

    Scenario: Only an auto-upload records it
      When alice uploads a file from the file picker
      Then the file has no source device

    Scenario: Renaming the device leaves it
      Given "IMG_1.jpg" was auto-uploaded from "Pixel"
      When the device is renamed to "Old phone"
      Then the source device of "IMG_1.jpg" is still "Pixel"

  Rule: The user decides about what is already in a folder

    Scenario: Leaving out existing files
      Given "Camera" holds "old.jpg"
      When alice adds "Camera" and turns "Upload existing files" off
      Then "old.jpg" is not uploaded
      But it is uploaded once it is edited

    Scenario: Uploading existing files
      Given "Camera" holds "old.jpg"
      When alice adds "Camera" and turns "Upload existing files" on
      Then "old.jpg" is uploaded

  Rule: A source that can't sync says why

    Scenario: The server folder was deleted
      Given the server folder "Photos" was deleted
      When a pass runs
      Then the source is paused and asks for another folder
      And the folder is not made again

    Scenario: The permission was taken back
      Given alice took back access to "Camera"
      When a pass runs
      Then the source shows the problem
      And it syncs again once access is back

  Rule: Uploads survive interruptions

    Scenario: An interrupted upload continues
      Given half of "video.mp4" was sent when the app stopped
      When the next pass runs
      Then it continues from what the server has

    Scenario: A file changed during its upload is not stored
      When "video.mp4" changes before its last chunk is sent
      Then the upload is cancelled
      And the new content is uploaded by a later pass

  # --- Settings -------------------------------------------------------------------------

  Rule: Sync only on Wi-Fi is on by default

    Scenario: No Wi-Fi
      Given "Sync only on Wi-Fi" is on
      And the phone is on mobile data
      When a photo appears in "Camera"
      Then nothing is uploaded until it is on Wi-Fi

    Scenario: Leaving Wi-Fi during an upload
      Given "Sync only on Wi-Fi" is on
      And an upload is under way
      When the phone leaves Wi-Fi
      Then the upload stops before its next chunk
      And it continues from there on Wi-Fi

    Scenario: Tapping a file is not automatic
      Given "Sync only on Wi-Fi" is on
      And the phone is on mobile data
      When alice taps a file to open it
      Then it downloads

  Rule: Sync in background is off by default

    Scenario: Background off
      Given "Sync in background" is off
      When the app is not open
      Then nothing syncs

    Scenario: Background on
      Given "Sync in background" is on
      When the app is not open
      Then the system runs passes when it allows

  # --- Signing out ----------------------------------------------------------------------

  Rule: Signing out ends the sync, not the files

    Scenario: Signing out
      When alice signs out
      Then uploads under way are cancelled
      And the sources are forgotten
      And the local folders are untouched
