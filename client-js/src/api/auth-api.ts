import { HttpMethod, RequestBuilder } from "./request-builder";
import { resolveApiUrl } from "./url-resolver";

interface LoginUserResponse {
    user_id: number;
    user_name: string;
    access_token: string;
}

export async function login_user(user_name: string, password: string): Promise<LoginUserResponse> {
    const url = resolveApiUrl("user", "login");
    const response = await RequestBuilder.create(url)
        .setMethod(HttpMethod.POST)
        .setJsonBody({
            user_name,
            password,
        })
        .build<LoginUserResponse>()
        .execute();
    return response;
}
