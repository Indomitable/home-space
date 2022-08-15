import jwt_decode from "jwt-decode";
import { HttpMethod, RequestBuilder } from "@/api/request-builder";
import { resolveApiUrl } from "@/api/url-resolver";
import type { InjectionKey } from "vue";

interface LoginUserResponse {
    user_id: number;
    user_name: string;
    access_token: string;
}

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

export interface User {
    userId: number;
    userName: string;
}

export class UserService {
    private static token_key = "app_user_context_key";

    async login(userName: string, password: string): Promise<void> {
        const url = resolveApiUrl("user", "login");
        const response = await RequestBuilder.create(url)
            .setMethod(HttpMethod.POST)
            .setJsonBody({
                user_name: userName,
                password,
            })
            .build<LoginUserResponse>()
            .execute();
        this.createUserContext(response);
    }

    logout(): void {
        sessionStorage.removeItem(UserService.token_key);
    }

    getLoggedUser(): User | null {
        const userContext = this.getUserContext();
        if (!userContext) {
            return null;
        }
        if (!this.validateUserContext(userContext)) {
            return null;
        }
        return {
            userId: userContext.user_id,
            userName: userContext.user_name,
        };
    }

    private createUserContext(loginResponse: LoginUserResponse): void {
        const decoded_jwt = jwt_decode<JWtTokenPayload>(loginResponse.access_token);
        const token_expirations_ms = decoded_jwt.exp * 1000;
        if (token_expirations_ms > Date.now() && decoded_jwt.user_id === loginResponse.user_id) {
            this.saveUserContext({
                user_id: loginResponse.user_id,
                user_name: loginResponse.user_name,
                access_token: {
                    token: loginResponse.access_token,
                    valid_until: decoded_jwt.exp * 1000,
                },
            });
        }
    }

    private saveUserContext(context: UserContext): void {
        sessionStorage.setItem(UserService.token_key, JSON.stringify(context));
    }

    private getUserContext(): UserContext | null {
        const val = sessionStorage.getItem(UserService.token_key);
        if (val) {
            return JSON.parse(val) as UserContext;
        }
        return null;
    }

    private validateUserContext(context: UserContext): boolean {
        return context.access_token.valid_until > Date.now();
    }
}

export const userServiceInjectionToken: InjectionKey<UserService> = Symbol("UserService");
