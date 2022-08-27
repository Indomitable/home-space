import type { InjectionKey } from "vue";

export class FormatterService {
    formatSize(sizeBytes: number): string {
        if (sizeBytes === 0) {
            return "0";
        }

        if (sizeBytes === 0) {
            return "0";
        }

        const kbyte = 1024;
        const mbyte = 1048576;
        const gbyte = 1073741824;
        const tbyte = 1099511627776;

        if (sizeBytes > tbyte) {
            return `${(sizeBytes / tbyte).toFixed(2)} TiB`;
        } else if (sizeBytes > gbyte) {
            return `${(sizeBytes / gbyte).toFixed(2)} GiB`;
        } else if (sizeBytes > mbyte) {
            return `${(sizeBytes / mbyte).toFixed(2)} MiB`;
        } else if (sizeBytes > kbyte) {
            return `${(sizeBytes / kbyte).toFixed(2)} KiB`;
        } else {
            return `${sizeBytes} byte(s)`;
        }
    }
}

export const formatterServiceInjectionToken: InjectionKey<FormatterService> = Symbol("FormatterService");
