import type { LoginUserResponse } from "@/api/auth-api";
import jwt_decode from "jwt-decode";

const token_key = "app_user_context_key";

interface JWtTokenPayload {
    user_id: number;
    user_name: string;
    exp: number;
}

interface UserContext {
    user_id: number;
    user_name: string;
    access_token: {
        token: string;
        valid_until: number;
    };
}

function saveUserContext(context: UserContext): void {
    sessionStorage.setItem(token_key, JSON.stringify(context));
}

export function getUserContext(): UserContext | null {
    const val = sessionStorage.getItem(token_key);
    if (val) {
        return JSON.parse(val) as UserContext;
    }
    return null;
}

export function isAuthenticated(): boolean {
    const context = getUserContext();
    if (!context) {
        return false;
    }
    // We can safty check valid_until, user can manually extend it on FE,
    // but this would not work on the backend which checks token too.
    return context.access_token.valid_until > Date.now();
}

export function authenticate(loginResponse: LoginUserResponse): void {
    const decoded_jwt = jwt_decode<JWtTokenPayload>(loginResponse.access_token);
    const token_expirations_ms = decoded_jwt.exp * 1000;
    if (token_expirations_ms > Date.now() && decoded_jwt.user_id === loginResponse.user_id) {
        saveUserContext({
            user_id: loginResponse.user_id,
            user_name: loginResponse.user_name,
            access_token: {
                token: loginResponse.access_token,
                valid_until: decoded_jwt.exp * 1000,
            },
        });
    }
}

export function logout(): void {
    sessionStorage.removeItem(token_key);
}
