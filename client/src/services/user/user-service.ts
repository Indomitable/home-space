import jwt_decode from "jwt-decode";
import type { InjectionKey } from "vue";
import { HttpMethod, RequestBuilder } from "@/api/request-builder";
import { resolveApiUrl } from "@/api/url-resolver";
import type { RequestInitVisitor } from "@/api/request-init-visitor";

interface LoginUserResponse {
    access_token: string;
}

interface JWtTokenPayload {
    exp: number;
}

interface UserContext {
    userName: string;
    access_token: {
        token: string;
        valid_until: number;
    };
}

export interface User {
    userName: string;
}

export class UserService implements RequestInitVisitor {
    private static token_key = "app_user_context_key";

    async login(userName: string, password: string): Promise<void> {
        const url = resolveApiUrl("auth", "login");
        const response = await RequestBuilder.create(url)
            .setMethod(HttpMethod.POST)
            .setJsonBody({
                userName,
                password,
            })
            .build<LoginUserResponse>()
            .execute();
        this.createUserContext(response, userName);
    }

    async register(userName: string, password: string) {
        const url = resolveApiUrl("auth", "register");
        const response = await RequestBuilder.create(url)
            .setMethod(HttpMethod.POST)
            .setJsonBody({
                userName,
                password,
            })
            .build<LoginUserResponse>()
            .execute();
        this.createUserContext(response, userName);
    }

    logout(): void {
        sessionStorage.removeItem(UserService.token_key);
    }

    getLoggedUser(): User | null {
        const userContext = this.getUserContext();
        if (!userContext || !this.validateUserContext(userContext)) {
            return null;
        }
        return {
            userName: userContext.userName,
        };
    }

    visit(requestInit: RequestInit): void {
        const userContext = this.getUserContext();
        if (!userContext || !this.validateUserContext(userContext)) {
            return;
        }
        (requestInit.headers as Record<string, string>)["Authorization"] = `Bearer ${userContext.access_token.token}`;
    }

    private createUserContext(loginResponse: LoginUserResponse, name: string): void {
        const decoded_jwt = jwt_decode<JWtTokenPayload>(loginResponse.access_token);
        const token_expirations_ms = decoded_jwt.exp * 1000;
        if (token_expirations_ms > Date.now()) {
            this.saveUserContext({
                userName: name,
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
