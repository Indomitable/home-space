# Trash

### Main responsibility:

- User deletes file it goes to trash
- User restores deleted file from trash.

- User deletes folder and whole folder goes to trash.
- User can restore folder from trash. User can restore whole folder no partial restore is possible.

### Use cases

1. No problems
    - File gets deleted and goes to trash
    - User restores the file from trash
    - File got removed from trash

2. User has created new file with same name after deleting first and tries to restore the delete one.
    - File gets deleted
    - User creates new file with same name and location
    - User tries to restore file:
        - Choice: Override current file: Yes/No
            - Yes - File got overridden:
                - Current active file goes to versioning folder
                - Deleted file becomes active
                - Deleted file removed from trash
            - No - Choice: Keep deleted file or remove from trash:
                - Keep: Do nothing. File stays in the trash
                - Delete: File is deleted from trash.

3. User creates and deletes multiple times same file.
    1. Tries to restore one of deleted file:
        - No active file: Use case 1
        - Has active file: Use case 2
    2. Tries to restore multiple same deleted files:
        - Multi Choices:
            - Cancel operation: Do nothing.
            - Choose which file to restore: Use case 3.1

    
                
