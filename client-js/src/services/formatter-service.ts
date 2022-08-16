import type { InjectionKey } from "vue";

export class FormatterService {
    formatSize(sizeBytes: number): string {
        if (sizeBytes === 0) {
            return "0";
        }

        const kbyte = 1024;
        const formats = ["", "KiB", "MiB", "GiB", "TiB", "PiB", "EiB", "ZiB", "YiB"];

        const formatIndex = Math.floor(Math.log(sizeBytes) / Math.log(kbyte));

        return `${parseFloat((sizeBytes / kbyte ** formatIndex).toFixed(2))} ${formats[formatIndex]}`;
    }
}

export const formatterServiceInjectionToken: InjectionKey<FormatterService> = Symbol("FormatterService");
