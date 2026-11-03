-- A device a user signs in from: one desktop install, one browser, one phone. The client makes
-- up the id and keeps it, so the device stays the same across sign-ins while its sessions come
-- and go. The id only labels where something came from; access still rests on the session.
-- It is keyed per user, so several users of one install each have their own row and nobody
-- can claim another user's device.
CREATE TABLE devices (
    user_id      UUID NOT NULL REFERENCES users (id) ON DELETE CASCADE,
    id           UUID NOT NULL,
    name         TEXT NOT NULL CHECK (length(name) BETWEEN 1 AND 100),
    platform     TEXT NOT NULL CHECK (length(platform) BETWEEN 1 AND 32),
    created_at   TIMESTAMPTZ NOT NULL DEFAULT now(),
    last_seen_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (user_id, id)
);

-- NULL for sessions of clients that do not name their device.
ALTER TABLE sessions ADD COLUMN device_id UUID;
ALTER TABLE sessions
    ADD CONSTRAINT sessions_device_fk
    FOREIGN KEY (user_id, device_id) REFERENCES devices (user_id, id) ON DELETE CASCADE;
CREATE INDEX sessions_device_idx ON sessions (user_id, device_id);

-- No foreign key: devices are only deleted with their user, whose uploads go too.
ALTER TABLE uploads ADD COLUMN device_id UUID;
CREATE INDEX uploads_device_idx ON uploads (owner_id, device_id);
