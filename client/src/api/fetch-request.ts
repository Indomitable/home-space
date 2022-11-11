// noinspection ES6MissingAwait

import { MimeTypes } from "./mime-types";
import { type UserContext, UserContextStorage } from "@/services/user/user-context";
import {NotAuthorizedError} from "@/errors/not-authorized-error";

export interface ServerError extends Error {
    code: number;
}

export class FetchRequest<TResponse> {
    constructor(
        private endpoint: string,
        private expectedFormat: XMLHttpRequestResponseType | "stream",
        private requestInit: RequestInit
    ) {}

    async execute(): Promise<TResponse> {
        const request = new Request(this.endpoint, this.requestInit);
        let response = await fetch(request);
        response = await this.handleRefreshToken(this.endpoint, this.requestInit, response);
        if (response.ok) {
            return this.handleOkResponse(response);
        }
        throw await this.handleError(response);
    }
    
    private async handleRefreshToken(originalUrl: string, originalInit: RequestInit, originalResponse: Response): Promise<Response> {
        if (originalResponse.headers.has("X-REFRESH-TOKEN") || originalResponse.headers.has("X-EXPIRED-TOKEN")) {
            // We need to refresh token.
            const context = UserContextStorage.getContext();
            if (!context) {
                return originalResponse;
            }
            const renewRequest = new Request("/api/auth/renew", {
                method: 'POST',
                body: context.refresh_token
            });
            let refreshTokenResponse = await fetch(renewRequest);
            if (refreshTokenResponse.ok) {
                const body = await refreshTokenResponse.json();
                const newContext: UserContext = {
                    ...context,
                    access_token: body.access_token,
                    refresh_token: body.refresh_token
                };
                UserContextStorage.saveContext(newContext);

                if (originalResponse.headers.has("X-EXPIRED-TOKEN")) {
                    // if action has expired due to the expired token
                    const retryRequest = new Request(originalUrl, originalInit);
                    // Replace token.
                    retryRequest.headers.set("Authorization", `Bearer ${newContext.access_token}`);
                    return await fetch(retryRequest);
                }
            } else {
                throw new NotAuthorizedError('Not Authorized. Token expired.');
            }
        }
        return originalResponse;
    }

    private async handleOkResponse(response: Response): Promise<TResponse> {
        switch (this.expectedFormat) {
            case "json": {
                const obj = await response.json();
                return obj as TResponse;
            }
            case "blob":
                return response.blob() as unknown as Promise<TResponse>;
            case "arraybuffer":
                return response.arrayBuffer() as unknown as Promise<TResponse>;
            case "text":
            case "document": {
                return response.text() as unknown as Promise<TResponse>;
            }
            case "stream": {
                return Promise.resolve({
                    stream: response.body,
                    length: +(response.headers.get("content-length") || "0"),
                }) as unknown as Promise<TResponse>;
            }
            default: {
                return undefined as unknown as TResponse;
            }
        }
    }

    private async handleError(response: Response): Promise<ServerError> {
        if (response.status === 401) {
            UserContextStorage.removeContext();
        }
        if (FetchRequest.isContentType(response, MimeTypes.Json)) {
            const json = await response.json();
            return {
                code: response.status,
                name: response.statusText,
                ...json,
            };
        }
        const body = await response.text();
        return {
            code: response.status,
            name: response.statusText,
            message: body || "Unknown error has occurred, please check logs!",
        };
    }

    private static isContentType(response: Response, contentType: string): boolean {
        const contentTypeHeader = response.headers.get("Content-Type");
        return !!(contentTypeHeader && contentTypeHeader.indexOf(contentType) > -1);
    }
}
