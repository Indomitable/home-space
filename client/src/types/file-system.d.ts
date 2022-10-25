// https://developer.mozilla.org/en-US/docs/Web/API/Window/showOpenFilePicker
// https://github.com/DefinitelyTyped/DefinitelyTyped/blob/master/types/wicg-file-system-access/index.d.ts

export {};

declare global {
    interface FileSystemHandle {
        readonly kind: "file" | "directory";
        readonly name: string;

        isSameEntry(other: FileSystemHandle): Promise<boolean>;
        queryPermission(descriptor?: FileSystemHandlePermissionDescriptor): Promise<PermissionState>;
        requestPermission(descriptor?: FileSystemHandlePermissionDescriptor): Promise<PermissionState>;
    }

    const FileSystemHandle: {
        prototype: FileSystemHandle;
        new (): FileSystemHandle;
    };

    type FileSystemHandleUnion = FileSystemFileHandle | HSFileSystemDirectoryHandle;

    interface FilePickerAcceptType {
        description?: string | undefined;
        accept: Record<string, string | string[]>;
    }

    interface FilePickerOptions {
        types?: FilePickerAcceptType[] | undefined;
        excludeAcceptAllOption?: boolean | undefined;
    }

    interface OpenFilePickerOptions extends FilePickerOptions {
        multiple?: boolean | undefined;
    }

    interface SaveFilePickerOptions extends FilePickerOptions {
        suggestedName?: string;
    }

    // eslint-disable-next-line @typescript-eslint/no-empty-interface
    interface DirectoryPickerOptions {}

    type FileSystemPermissionMode = "read" | "readwrite";

    interface FileSystemPermissionDescriptor extends PermissionDescriptor {
        handle: FileSystemHandle;
        mode?: FileSystemPermissionMode | undefined;
    }

    interface FileSystemHandlePermissionDescriptor {
        mode?: FileSystemPermissionMode | undefined;
    }

    interface FileSystemCreateWritableOptions {
        keepExistingData?: boolean | undefined;
    }

    interface FileSystemGetFileOptions {
        create?: boolean | undefined;
    }

    interface FileSystemGetDirectoryOptions {
        create?: boolean | undefined;
    }

    interface FileSystemRemoveOptions {
        recursive?: boolean | undefined;
    }

    type WriteParams =
        | { type: "write"; position?: number | undefined; data: BufferSource | Blob | string }
        | { type: "seek"; position: number }
        | { type: "truncate"; size: number };

    type FileSystemWriteChunkType = BufferSource | Blob | string | WriteParams;

    class FileSystemWritableFileStream extends WritableStream {
        write(data: FileSystemWriteChunkType): Promise<void>;
        seek(position: number): Promise<void>;
        truncate(size: number): Promise<void>;
    }

    interface FileSystemFileHandle extends FileSystemHandle {
        readonly kind: "file";
        getFile(): Promise<File>;
        createWritable(options?: FileSystemCreateWritableOptions): Promise<FileSystemWritableFileStream>;
    }

    const FileSystemFileHandle: {
        prototype: FileSystemFileHandle;
        new (): FileSystemFileHandle;
    };

    interface HSFileSystemDirectoryHandle extends FileSystemHandle {
        readonly kind: "directory";
        getDirectoryHandle(name: string, options?: FileSystemGetDirectoryOptions): Promise<HSFileSystemDirectoryHandle>;
        getFileHandle(name: string, options?: FileSystemGetFileOptions): Promise<FileSystemFileHandle>;
        removeEntry(name: string, options?: FileSystemRemoveOptions): Promise<void>;
        resolve(possibleDescendant: FileSystemHandle): Promise<string[] | null>;
        keys(): AsyncIterableIterator<string>;
        values(): AsyncIterableIterator<HSFileSystemDirectoryHandle | FileSystemFileHandle>;
        entries(): AsyncIterableIterator<[string, HSFileSystemDirectoryHandle | FileSystemFileHandle]>;
        [Symbol.asyncIterator]: HSFileSystemDirectoryHandle["entries"];
    }

    const FileSystemDirectoryHandle: {
        prototype: HSFileSystemDirectoryHandle;
        new (): HSFileSystemDirectoryHandle;
    };

    function showOpenFilePicker(options?: OpenFilePickerOptions & { multiple?: false | undefined }): Promise<[FileSystemFileHandle]>;
    function showOpenFilePicker(options?: OpenFilePickerOptions): Promise<FileSystemFileHandle[]>;
    function showDirectoryPicker(options?: DirectoryPickerOptions): Promise<HSFileSystemDirectoryHandle>;
    function showSaveFilePicker(options: SaveFilePickerOptions): Promise<FileSystemFileHandle>;
}
