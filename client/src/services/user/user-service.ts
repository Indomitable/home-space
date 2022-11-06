// import jwt_decode from "jwt-decode";
import type { InjectionKey } from "vue";
import { HttpMethod, RequestBuilder } from "@/api/request-builder";
import { resolveApiUrl } from "@/api/url-resolver";
import type { RequestInitVisitor } from "@/api/request-init-visitor";
import {type UserContext, UserContextStorage} from "@/services/user/user-context";

interface LoginUserResponse {
    access_token: string;
    refresh_token: string;
}

// interface JWtTokenPayload {
//     exp: number;
// }

export interface User {
    userName: string;
}

export class UserService implements RequestInitVisitor {
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
        UserContextStorage.removeContext();
    }

    getLoggedUser(): User | null {
        const userContext = UserContextStorage.getContext();
        if (!userContext) {
            return null;
        }
        return {
            userName: userContext.userName,
        };
    }

    visit(requestInit: RequestInit): void {
        const userContext = UserContextStorage.getContext();
        if (!userContext) {
            return;
        }
        (requestInit.headers as Record<string, string>)["Authorization"] = `Bearer ${userContext.access_token}`;
    }

    private createUserContext(loginResponse: LoginUserResponse, name: string): void {
        // const decoded_jwt = jwt_decode<JWtTokenPayload>(loginResponse.access_token);
        // const token_expirations_ms = decoded_jwt.exp * 1000;
        // if (token_expirations_ms > Date.now()) {
            UserContextStorage.saveContext({
                userName: name,
                access_token: loginResponse.access_token,
                refresh_token: loginResponse.refresh_token
            });
        // }
    }

    // private validateUserContext(context: UserContext): boolean {
    //     return context.access_token.valid_until > Date.now();
    // }
}

export const userServiceInjectionToken: InjectionKey<UserService> = Symbol("UserService");
