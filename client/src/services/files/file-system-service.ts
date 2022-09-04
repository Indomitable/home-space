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

    async saveFileStream(
        data: { stream: ReadableStream<Uint8Array>; length: number },
        suggestedName: string,
        acceptTypes: FilePickerAcceptType[]
    ): Promise<void> {
        const { showSaveFilePicker } = window;
        if (typeof showSaveFilePicker === "undefined") {
            // Firefox doesn't support showSaveFilePicker, so we need to put file in memory first
            // This is not optimal for bigger files can be too slow.
            let blob: Blob | undefined;
            let chunks: Uint8Array[] | null = [];
            // We can read chunks in array or using WritableStream
            // Writable Stream is fun :)
            const writableStream = new WritableStream({
                write: chunk => {
                    chunks!.push(chunk);
                },
                close: () => {
                    blob = new Blob(chunks!);
                },
            });
            await data.stream.pipeTo(writableStream);
            if (blob) {
                const downloadLink = document.createElement("a");
                const url = window.URL.createObjectURL(blob);
                downloadLink.href = url;
                downloadLink.download = suggestedName;
                downloadLink.click();
                window.URL.revokeObjectURL(url);
                chunks = null;
            }
        }
        const options: SaveFilePickerOptions = {
            types: acceptTypes,
            excludeAcceptAllOption: false,
            suggestedName,
        };

        const handle = await showSaveFilePicker(options);
        const writable = await handle.createWritable();
        await data.stream.pipeTo(writable);
        // We can add download progress using following code, for now use pipeTo
        // const reader = data.stream.getReader();
        // const writer = writable.getWriter();
        // let bytesWritten = 0;
        // for (let chunk = await reader.read(); !chunk.done; chunk = await reader.read()) {
        //     if (data.length > 0) {
        //         bytesWritten += chunk.value.length;
        //         console.debug(`Write ${bytesWritten} bytes of ${data.length} bytes`);
        //     }
        //     await writer.write(chunk.value);
        // }
        // await writer.close();
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
