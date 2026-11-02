-- Users who sign in through an external identity provider have no local password.
ALTER TABLE users ALTER COLUMN password_hash DROP NOT NULL;

-- Links a local user to an account at an OpenID Connect provider. Users are matched on
-- (issuer, subject) only, never on e-mail, which a provider may let people change.
CREATE TABLE user_identities (
    issuer     TEXT NOT NULL,
    subject    TEXT NOT NULL,
    user_id    UUID NOT NULL REFERENCES users (id) ON DELETE CASCADE,
    email      TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (issuer, subject)
);

CREATE INDEX user_identities_user_idx ON user_identities (user_id);

-- A sign-in that was sent to the provider and has not come back yet.
CREATE TABLE oidc_pending (
    state_hash        BYTEA PRIMARY KEY,
    nonce             TEXT NOT NULL,
    idp_pkce_verifier TEXT NOT NULL,
    client_redirect   TEXT NOT NULL,
    client_challenge  TEXT NOT NULL,
    expires_at        TIMESTAMPTZ NOT NULL
);

-- A one-time code handed to the client after a successful sign-in, exchanged for tokens.
CREATE TABLE oidc_codes (
    code_hash        BYTEA PRIMARY KEY,
    user_id          UUID NOT NULL REFERENCES users (id) ON DELETE CASCADE,
    client_challenge TEXT NOT NULL,
    expires_at       TIMESTAMPTZ NOT NULL
);
