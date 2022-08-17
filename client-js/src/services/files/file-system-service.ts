import type { InjectionKey } from "vue";

export class FileSystemService {
    /*
     * Example:
     * saveFile(blob, "NiceImage.png", [
     *  {
     *      description: "Images",
     *      accept: { "image/*": ['.png', '.gif', '.jpeg', '.jpg']
     *  }
     * ])
     */
    async saveFile(blob: Blob, suggestedName: string, acceptTypes: FilePickerAcceptType[]): Promise<void> {
        const { showSaveFilePicker } = window;
        if (typeof showSaveFilePicker === "undefined") {
            // Firefox
            const downloadLink = document.createElement("a");
            const url = window.URL.createObjectURL(blob);
            downloadLink.href = url;
            downloadLink.download = suggestedName;
            downloadLink.click();
            window.URL.revokeObjectURL(url);
        } else {
            const options: SaveFilePickerOptions = {
                types: acceptTypes,
                excludeAcceptAllOption: false,
                suggestedName,
            };

            const handle = await showSaveFilePicker(options);
            const writable = await handle.createWritable();
            await writable.write(blob);
            await writable.close();
        }
    }

    async *loadFiles(acceptTypes: FilePickerAcceptType[]): AsyncGenerator<File> {
        const { showOpenFilePicker } = window;
        if (typeof showOpenFilePicker === "undefined") {
            // Firefox
            const files = await new Promise<File[]>((resolve, reject) => {
                const input = document.createElement("input") as HTMLInputElement;
                input.type = "file";
                input.accept = acceptTypes.reduce((aggr, accept) => {
                    return (
                        aggr +
                        Object.values(accept).reduce((aggr: string, val) => {
                            const concat = Array.isArray(val) ? val.join(",") : val;
                            return aggr + concat;
                        }, "")
                    );
                }, "");
                input.style.display = "none";
                input.multiple = true;
                input.onchange = () => {
                    // document.body.removeChild(input);
                    if (input.files) {
                        const files: File[] = [];
                        for (let i = 0; i < input.files.length; i++) {
                            files.push(input.files[i]);
                        }
                        resolve(files);
                    } else {
                        reject(new Error("No file selected"));
                    }
                };
                input.click();
            });
            for (const file of files) {
                yield await Promise.resolve(file);
            }
        } else {
            const handlers = await showOpenFilePicker({
                types: acceptTypes,
                multiple: true,
                excludeAcceptAllOption: false,
            });
            for (const handler of handlers) {
                yield await handler.getFile();
            }
        }
    }
}

export const fileSystemServiceInjectionToken: InjectionKey<FileSystemService> = Symbol("FileSystemService");
