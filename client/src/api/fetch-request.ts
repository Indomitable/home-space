import { MimeTypes } from "./mime-types";

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
        const response = await fetch(request);
        if (response.ok) {
            return this.handleOkResponse(response);
        }
        const error = await this.readError(response);
        throw error;
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

    private async readError(response: Response): Promise<ServerError> {
        if (FetchRequest.isContentType(response, MimeTypes.Json)) {
            const { message } = await response.json();
            return {
                code: response.status,
                name: response.statusText,
                message,
            };
        }
        const body = await response.text();
        return {
            code: response.status,
            name: response.statusText,
            message: body || "Unknwon error has occured, please check logs!",
        };
    }

    private static isContentType(response: Response, contentType: string): boolean {
        const contentTypeHeader = response.headers.get("Content-Type");
        return !!(contentTypeHeader && contentTypeHeader.indexOf(contentType) > -1);
    }
}
