-- A session is one signed-in device. Access tokens and tickets name their session and are
-- only accepted while its row exists, so a leaked signing secret alone cannot forge access:
-- sessions are created after a verified password or provider login, never from a token.
CREATE TABLE sessions (
    id         UUID PRIMARY KEY,
    user_id    UUID NOT NULL REFERENCES users (id) ON DELETE CASCADE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    expires_at TIMESTAMPTZ NOT NULL
);

CREATE INDEX sessions_user_idx ON sessions (user_id);

-- A refresh token family is a session. Drop the tokens of families that can no longer be
-- used, then give every remaining family a session so nobody is signed out by this upgrade.
DELETE FROM refresh_tokens r
WHERE NOT EXISTS (
    SELECT 1 FROM refresh_tokens live
    WHERE live.family_id = r.family_id AND live.revoked_at IS NULL AND live.expires_at > now()
);

INSERT INTO sessions (id, user_id, expires_at)
SELECT family_id, (array_agg(user_id))[1], max(expires_at)
FROM refresh_tokens
GROUP BY family_id;

ALTER TABLE refresh_tokens
    ADD CONSTRAINT refresh_tokens_session_fk
    FOREIGN KEY (family_id) REFERENCES sessions (id) ON DELETE CASCADE;
