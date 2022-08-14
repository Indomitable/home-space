import jwt_decode from "jwt-decode";

const token_key = "app_user_context_key";

interface JWtTokenPayload {
    user_id: number;
    user_name: string;
    exp: number;
}

export function saveToken(access_token: string): void {
    sessionStorage.setItem(token_key, access_token);
}

export function getToken(): string | null {
    return sessionStorage.getItem(token_key);
}

export function isAuthenticated(): boolean {
    const token = getToken();
    if (!token) {
        return false;
    }
    const decoded_jwt = jwt_decode<JWtTokenPayload>(token);
    const now = Date.now();
    return decoded_jwt.exp * 1000 > now; // exp is in seconds.
}
