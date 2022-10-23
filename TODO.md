TODO:

### File operations:
- [ ] Rewrite current frontend in VueJs: The initial frontend was in Yew using Rust but developing is quite slow. Using VueJS makes FE developing faster.
  - [x] Create folder
  - [x] Upload file
  - [ ] Drag and drop file in the list view.
  - [x] Copy files
  - [x] Move files
  - [ ] Delete files
  - [x] Rename files
  - [x] Context menu
  - [ ] Folder size
  - [ ] Account register

### Pages
- [ ] List of files page. Initial release load all files in folders.
    - [ ] Continuously loading
- [ ] Implement Favorites page: list of favorite items
- [ ] Implement Recent page
    - [ ] List of last modified items
- [ ] Implement Shared page.
    - [ ] Ability to access files without user name and password ( link or password security )
    - [ ] Share files with other users. Different access: View or Edit
- [ ] Implement Trash page: view deleted files.
    - [ ] Ability to restore deleted files. 
- [ ] Settings page
- [ ] Account page:
  - [ ] Change password
  - [ ] Add/Edit 2FA 

### Single Features 
- [ ] Ability to view/download and restore previous versions. 
- [ ] Search
- [ ] Ability to create file templates for new files.
- [ ] Create custom file views
    - [ ] Simple text editor 
    - [ ] Code editor (highlighting, using Monaco/VsCode)
    - [ ] Image viewer 
    - [ ] Video viewer
- [ ] Different file views
  - [ ] Grid view
  - [ ] Image gallery
- [ ] External locations: Ability to attach external folder
    - [ ] Readonly mode: View and download files in it.
    - [ ] Edit mode: Copy/Move/Rename files
    - [ ] Move files between managed and external locations.
- [ ] Process indication. Upload/Copy/Move file indicator. How should we use websocket to upload file and show progress? 

### Security
- [ ] Move JWT to HTTP only, SameSite and Secure cookie: This will give ability to open files in the browser with direct links.
- [ ] Save JWT tokens in database: Revoke token on logout
- [ ] More ways to login:
    - [ ] Support of two factor authentications
      - [ ] 2FA Authentication apps
      - [ ] Passwordless login: WebAuthn using hardware tokens (YubiKey)

### Mobile
- [ ] Phone and Tablet layout. ( Responsive design )
- [ ] Android native app
    - [ ] File sync
- [ ] iPhone native app
    - [ ] File sync

### Extra Services
- [ ] Develop extra service which clean orphan versions, the temp files and old items in the trash.