import { FetchRequest } from "./fetch-request";
import { MimeTypes } from "./mime-types";

export enum HttpMethod {
    GET = "GET",
    POST = "POST",
    DELETE = "DELETE",
    PUT = "PUT",
    HEAD = "HEAD",
    PATCH = "PATCH",
}

export class RequestBuilder {
    private requestInit: RequestInit = {
        headers: {},
    };

    public static create(endpoint: string): RequestBuilder {
        return new RequestBuilder(endpoint);
    }

    constructor(private endpoint: string) {}

    setMethod(method: HttpMethod): RequestBuilder {
        this.requestInit.method = method;
        return this;
    }

    setBody(body: BodyInit): RequestBuilder {
        this.requestInit.body = body;
        return this;
    }

    setJsonBody<TBody>(body: TBody): RequestBuilder {
        (this.requestInit.headers as Record<string, string>)["Content-Type"] = MimeTypes.Json;
        return this.setBody(JSON.stringify(body));
    }

    build<TResponse>(): FetchRequest<TResponse>;
    build<TResponse>(expectedResponseType: "json"): FetchRequest<TResponse>;
    build(expectedResponseType: "blob"): FetchRequest<Blob>;
    build(expectedResponseType: "arraybuffer"): FetchRequest<ArrayBuffer>;
    build(expectedResponseType: "text"): FetchRequest<string>;
    build(expectedResponseType: "document"): FetchRequest<string>;
    build(expectedResponseType: ""): FetchRequest<void>;
    build(expectedResponseType: XMLHttpRequestResponseType = "json"): FetchRequest<unknown> {
        if (expectedResponseType === "json") {
            (this.requestInit.headers as Record<string, string>).accept = MimeTypes.Json;
        }
        return new FetchRequest(this.endpoint, expectedResponseType, this.requestInit);
    }
}
