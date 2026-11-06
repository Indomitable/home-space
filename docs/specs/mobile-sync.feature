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

    Scenario: Editing a photo whose name another file took keeps both
      Given "IMG_1.jpg" was uploaded
      And it was moved to "Archive" on the server
      And a different file "IMG_1.jpg" was added to "Photos"
      When the local file is edited
      Then "Photos/IMG_1.jpg" is unchanged
      And a new file "Photos/IMG_1 (1).jpg" exists on the server

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

    Scenario: Two phones with the same name keep their own files through edits
      Given two phones each uploaded a different "IMG_1.jpg" to "Photos"
      When each edits its own file
      Then "Photos/IMG_1.jpg" and "Photos/IMG_1 (1).jpg" each have a new version

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

  # --- Offline ---------------------------------------------------------------------------

  Rule: What is kept on the phone syncs both ways

    Scenario: Opening a file keeps it
      When "note.txt" is tapped
      Then the file opens
      And it is kept on the phone and shows as available offline

    Scenario: Making a folder available offline
      When "Docs" is made available offline
      Then its files and subfolders download
      And everything in it shows as available offline

    Scenario: A change on the server arrives
      Given "note.txt" is kept on the phone
      When it changes on the server
      Then the copy on the phone is updated

    Scenario: A rename on the server arrives
      Given "note.txt" is kept on the phone
      When it is renamed on the server
      Then the copy on the phone has the new name

    Scenario: An edit goes up
      Given "note.txt" is kept on the phone
      When it is edited in another app or in Quick Look
      Then the server holds the edit as the current version
      And the previous content is a version

    Scenario: An edit and a rename on the server
      Given "note.txt" is kept on the phone
      When it is edited on the phone and then renamed on the server
      Then the server holds the edit under the new name

    Scenario: An edit in a kept folder and a rename on the server
      Given the folder "Docs" with "note.txt" is kept on the phone
      When "note.txt" is edited on the phone and then renamed on the server
      Then the server holds the edit under the new name
      And no file remains under the old name

    Scenario: Changes on both sides
      Given "note.txt" is kept on the phone
      When it changes on both sides
      Then the later change is current
      And the other is a version

    Scenario: A tap does not overwrite an edit
      Given "note.txt" has an edit not uploaded yet
      And newer content is on the server
      When "note.txt" is tapped
      Then the edit opens

  Rule: Only the app deletes, and only through the server

    Scenario: A missing local file comes back
      Given "Docs/a.txt" is kept on the phone
      When its local file goes missing
      Then it downloads again
      And it stays on the server

    Scenario: A file deleted on the server
      Given "Docs/a.txt" is kept on the phone
      When it is deleted on the server
      Then the copy on the phone is deleted

    Scenario: Something kept is deleted on the server
      Given "note.txt" is kept for its own sake
      When it is deleted on the server
      Then its copy is deleted
      And it is no longer kept

    Scenario: Something kept is deleted on the server, with an edit not uploaded
      Given "note.txt" has an edit not uploaded yet
      When it is deleted on the server
      Then the edit is uploaded to Lost & Found
      And the copy is deleted

  Rule: Removing warns about edits not uploaded yet

    Scenario: Removing an offline copy
      Given "note.txt" is kept on the phone with nothing to upload
      When its offline copy is removed
      Then it is deleted from the phone and stays on the server

    Scenario: Removing a copy with an edit not uploaded
      Given "note.txt" has an edit not uploaded yet
      When its offline copy is removed
      Then the user is warned, and may cancel

    Scenario: Signing out with an edit not uploaded
      Given "note.txt" has an edit not uploaded yet
      When the user signs out
      Then the user is warned, and may cancel

    Scenario: A file in a kept folder has no removal of its own
      Given "Docs" is made available offline
      Then "Docs/a.txt" offers no removal, only "Docs" does

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

  Rule: Signing out ends the sync, not the local source files

    Scenario: Signing out
      When alice signs out
      Then uploads under way are cancelled
      And the sources are forgotten
      And the local folders are untouched

    Scenario: Signing out removes offline copies
      Given "note.txt" is kept on the phone
      When the user signs out
      Then the offline copies are deleted from the phone

  Rule: A photo source isn't read in full once it can say what changed

    Background:
      Given the camera uploads to the server folder "Photos"
      And a pass has dealt with everything in it

    Scenario: A new photo needs no listing
      When a new photo is taken
      Then the next pass asks only what changed, and uploads it

    Scenario: A photo that couldn't go yet isn't forgotten
      Given a new photo whose upload failed
      When nothing else changes
      Then the next pass tries it again

    Scenario: The platform can't answer
      Given the change token has expired, or a memory card came or went
      When a pass runs
      Then it lists everything, and files already sent stay as they are

    Scenario: A listing now and then
      Given the last full listing was over 7 days ago
      When a pass runs
      Then it lists everything, which finds anything the platform failed to report

    Scenario: Access to selected photos only
      Given the user allowed only some photos
      When a pass runs
      Then it lists them, since one added to the selection isn't reported as a change
