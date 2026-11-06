-- The file a finished upload may become a version of; see `CreateUpload`. No foreign key: the file
-- may be gone by the time the upload finishes, and then the upload is simply a file of its own.
ALTER TABLE uploads ADD COLUMN version_of UUID;
