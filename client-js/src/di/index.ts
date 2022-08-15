import type { InjectionKey } from "vue";

export interface Provider {
    <T>(key: InjectionKey<T>, value: T): void;
}

export interface Injector {
    <T>(key: InjectionKey<T>): T | undefined;
}
