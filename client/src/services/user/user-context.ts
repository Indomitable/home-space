export interface UserContext {
    userName: string;
    access_token: string;
    refresh_token: string;
}

export class UserContextStorage {
    private static token_key = "app_user_context_key";

    public static getContext(): UserContext | null {
        const val = sessionStorage.getItem(UserContextStorage.token_key);
        if (val) {
            return JSON.parse(val) as UserContext;
        }
        return null;
    }
    
    public static saveContext(context: UserContext) {
        sessionStorage.setItem(UserContextStorage.token_key, JSON.stringify(context));
    }
    
    public static removeContext() {
        sessionStorage.removeItem(UserContextStorage.token_key);
    }
}