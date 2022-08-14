import { getUserContext } from "@/auth/authentication";
import { defineStore } from "pinia";

export interface UserState {
    userId: number;
    userName: string;
}

export const useUserStore = defineStore("user", {
    state: (): UserState => {
        const userContext = getUserContext();
        if (!userContext) {
            return {
                userId: 0,
                userName: "",
            };
        }
        return {
            userId: userContext.user_id,
            userName: userContext.user_name,
        };
    },
    actions: {
        setUser(userId: number, userName: string) {
            this.userId = userId;
            this.userName = userName;
        },
    },
});
