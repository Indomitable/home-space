//! A fake OpenID Connect provider for tests: discovery, signing keys and a token endpoint.
//!
//! The RSA key below is a throwaway used only here; it protects nothing.

use jsonwebtoken::{Algorithm, EncodingKey, Header, encode};
use serde_json::{Value, json};
use wiremock::{
    Mock, MockServer, ResponseTemplate,
    matchers::{method, path},
};

const PRIVATE_KEY: &str = "-----BEGIN RSA PRIVATE KEY-----
MIIEogIBAAKCAQEAt3bODH32IBvOvKJS7S0RFtSdZi5r4ynIP+RP6r4HiDik+2i8
rg1ywqkq9TpiyMqy/+i01lfoO4wpvIh4jt5+3UUub/GxwzuwwvWEFPMx5/Dj6hcJ
CdJBZSlhXRpDwIHgl7DKlvSsINk8hWFFCpcMJ1pdr0Cj8o3gWnHRb8laB56hRBY8
o5bRrfTnKhHbJtXf5+y6nRWDwn4Kb1FaVkXyABVujp4YSbrestpjCRe88QPVw/+E
bR8NnfLuBjQaITa3It/D7PtYIboGjCF/Kf1qcvXQQXqpoby2Uxf4viz4rSdOn0Gy
1n2zCmBfYlpK3LaoRVxqn09VI/LAVlIQkCG09QIDAQABAoIBAA82+BGLJK03AScd
oxjuQtuM2FzP/ryBC41Y5tPF3KQMZoo+/l7E2L6R/vEFXYnfXYbhipW4BpdhoVh0
TOM0SnsZMKex+JcDTT5t0/1Uyc+9Z3lTXbmFyUyVaPCqq/G8E58g1zw7oEa9HUrG
bU5ahnSBBwHwr2f/Ihr+E2tc3G2DMuJL5L2+SLS71n3FCPDHWqryQ3yTjekd1HM9
ew6xaNK/rsdNgSRcv12wosAHT4QEC9P2roC3zdPdE16faQONHPxVKXZzngwNuwJG
VllmIsTaaZxAPoPCWxdSqI9O2h/NLzKeUsQfSkhX5YPBcdsQrlOAR7Me3k3jxqJx
rQhHeEECgYEA/aflyuSMMVHlR5CTFUnX9aE2gHX/hPkI184Qiby2eXCxFj/WMWyC
lWoR1y3KSzDmhOGCrZ0WO4VGj8aESfXA2ydzY94S4pJF/Rasl5pY2kzm4lNJLBsl
NSpNzf5DB5niZ88aiL5kC7Q6PlAguvuLALbha5kqV33xd3zc7gVDNTECgYEAuSjY
vQhUyoPiQOVOCS5SxanpgD+HrUTIirUiLzhFx2bQLxgU/K2/AtiWCDjYLnkm+q/4
XbG7vGXg1DTpJ098zhzz/epmbxNlktr+FWdlD7+yZiag5dtVqa6qbPoQx9SDvGIl
HvUBqNxtuyiZqyy+gB+e4OgpKbNItrpzIlstmwUCgYBpx1Sa0chs4NwehfnwTMUU
l9AkrhCKoVfAtxxKdDocVpRi1Og74CZsrLkX5O6RgKkBDRHz2Khm1eNhHcYZGbT9
L2x9XyS8+4bYTlZXOnSHRL6b72N6Uy+nBidAGiJVzySNad6E2/DYuKX5Smz7zN5g
w+MpfcwchFboH1f6opMuYQKBgAYQIli+RxiCjQwW9iEy1R8H20+JxQWVdQOlvsLR
Eufeo/6GyVkkz1ZmhTMruVh8OxyGWpmm0kDvDRely+Svo6SUKiEsZcJJXlAav6sv
wY1sd4kExswADKQLxiTfY2FHwjCO/iwvVaiT3zv7rlx+kJZdcrRmpcJpzn29STvc
xHeJAoGASelXQB5tQ4zXi4THd01hmUIpHwTqPKd5rrvNx8JH0dYNyNQ2RoKbxTuL
wAe9Tmv4kuPCmfOu2yesY+Zp+V2bTdxCQ0QuCespgCjxZ410iwdNCVD1AF0JS/9L
lomgwgUsAdqwAM5gVqdxP+1p//syeZSrAgS6E2OWtHtzr9TAvqY=
-----END RSA PRIVATE KEY-----
";
const MODULUS: &str = "t3bODH32IBvOvKJS7S0RFtSdZi5r4ynIP-RP6r4HiDik-2i8rg1ywqkq9TpiyMqy_-i01lfoO4wpvIh4jt5-3UUub_GxwzuwwvWEFPMx5_Dj6hcJCdJBZSlhXRpDwIHgl7DKlvSsINk8hWFFCpcMJ1pdr0Cj8o3gWnHRb8laB56hRBY8o5bRrfTnKhHbJtXf5-y6nRWDwn4Kb1FaVkXyABVujp4YSbrestpjCRe88QPVw_-EbR8NnfLuBjQaITa3It_D7PtYIboGjCF_Kf1qcvXQQXqpoby2Uxf4viz4rSdOn0Gy1n2zCmBfYlpK3LaoRVxqn09VI_LAVlIQkCG09Q";
const KEY_ID: &str = "test-key";
pub const CLIENT_ID: &str = "homespace-test";

pub struct FakeIdp {
    server: MockServer,
}

/// What the provider puts in the ID token it returns.
pub struct IdToken {
    pub subject: &'static str,
    pub nonce: String,
    pub audience: &'static str,
    /// Seconds from now until the token expires; negative for an expired token.
    pub expires_in: i64,
    /// Extra claims, e.g. `preferred_username` or `groups`.
    pub extra: Value,
}

impl IdToken {
    pub fn new(subject: &'static str, nonce: &str) -> Self {
        Self {
            subject,
            nonce: nonce.to_owned(),
            audience: CLIENT_ID,
            expires_in: 300,
            extra: json!({}),
        }
    }

    pub fn with(mut self, extra: Value) -> Self {
        self.extra = extra;
        self
    }
}

impl FakeIdp {
    pub async fn start() -> Self {
        let idp = Self {
            server: MockServer::start().await,
        };
        idp.mount_discovery().await;
        idp
    }

    pub fn issuer(&self) -> String {
        self.server.uri()
    }

    /// Makes the token endpoint answer with an ID token for `token` (replacing earlier answers).
    pub async fn respond_with(&self, token: IdToken) {
        self.server.reset().await;
        self.mount_discovery().await;
        let id_token = self.sign(&token);
        Mock::given(method("POST"))
            .and(path("/token"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "access_token": "provider-access-token",
                "token_type": "bearer",
                "expires_in": 300,
                "id_token": id_token,
            })))
            .mount(&self.server)
            .await;
    }

    /// Makes the token endpoint reject every code.
    pub async fn reject_codes(&self) {
        self.server.reset().await;
        self.mount_discovery().await;
        Mock::given(method("POST"))
            .and(path("/token"))
            .respond_with(
                ResponseTemplate::new(400).set_body_json(json!({ "error": "invalid_grant" })),
            )
            .mount(&self.server)
            .await;
    }

    async fn mount_discovery(&self) {
        let issuer = self.issuer();
        Mock::given(method("GET"))
            .and(path("/.well-known/openid-configuration"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "issuer": issuer,
                "authorization_endpoint": format!("{issuer}/authorize"),
                "token_endpoint": format!("{issuer}/token"),
                "jwks_uri": format!("{issuer}/jwks"),
                "response_types_supported": ["code"],
                "subject_types_supported": ["public"],
                "id_token_signing_alg_values_supported": ["RS256"],
            })))
            .mount(&self.server)
            .await;
        Mock::given(method("GET"))
            .and(path("/jwks"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "keys": [{
                    "kty": "RSA",
                    "use": "sig",
                    "alg": "RS256",
                    "kid": KEY_ID,
                    "n": MODULUS,
                    "e": "AQAB",
                }]
            })))
            .mount(&self.server)
            .await;
    }

    fn sign(&self, token: &IdToken) -> String {
        let now = time::OffsetDateTime::now_utc().unix_timestamp();
        let mut claims = json!({
            "iss": self.issuer(),
            "sub": token.subject,
            "aud": token.audience,
            "iat": now,
            "exp": now + token.expires_in,
            "nonce": token.nonce,
            // Auth0 sends this as an ISO 8601 string rather than seconds since the epoch.
            "updated_at": "2026-11-02T17:25:53.422Z",
        });
        if let (Some(claims), Some(extra)) = (claims.as_object_mut(), token.extra.as_object()) {
            claims.extend(extra.clone());
        }
        let mut header = Header::new(Algorithm::RS256);
        header.kid = Some(KEY_ID.to_owned());
        encode(
            &header,
            &claims,
            &EncodingKey::from_rsa_pem(PRIVATE_KEY.as_bytes()).expect("test key"),
        )
        .expect("sign id token")
    }
}
