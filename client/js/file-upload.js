function createTemplate(supports_open_dialog) {
    const template = document.createElement('template');
    
    const hint = supports_open_dialog
        ? `<span>Drag and drop file(s) or folder(s) you want to upload or click <a class="select-file">select</a></span>`
        : `<span>Drag and drop file(s) or folder(s) you want to upload or <input type="file" />
            </span>`;
    template.innerHTML = `
    <style>
        :host {
            display: grid;
            height: 100%;
            place-content: center;
        }

        :host(.drag-over) {
            background: #EFF6EA;
            border: 2px dashed #30760b;
        }

        .select-file {
            cursor: pointer;
            color: red;
        }
    </style>
    <div class="drop-hint">
        ${hint}
    </div>
  `;

    return template.content.cloneNode(true);
}

class FileUpload extends HTMLElement {
    #onDragOver;
    #onDragLeave;
    #onDrop;
    #supportsOpenDialog;
    #onSelectFile;
    #parentId;

    constructor() {
        super();
        const root = this.attachShadow({ mode: 'open' });

        this.#onDragOver = (event) => {
            if (event.dataTransfer && event.dataTransfer.items.length > 0) {
                const items = Array.from(event.dataTransfer.items);
                if (items.some(i => i.kind == 'file')) {
                    this.classList.toggle('drag-over', true);
                    event.preventDefault();
                }
            }
        };
        this.#onDragLeave = () => {
            this.classList.toggle('drag-over', false);
        };
        this.#onDrop = async (event) => {
            event.preventDefault();
            if (event.dataTransfer
                && event.dataTransfer.items.length > 0
                && event.dataTransfer.files.length > 0) {
                const items = Array.from(event.dataTransfer.items);
                for (const item of items.filter(i => i.kind == 'file')) {
                    if (typeof item.webkitGetAsEntry  == 'function') {
                        // Non stardart way but supported from Chrome and Firefox
                        this.#readFileSystemEntry(item);
                    }

                    if (typeof item.getAsFileSystemHandle == 'function') {
                        // Stardard way but supported only from Chrome and it is async.
                        this.#readFileSystemHandlers(item);
                    }
                }
            }
            this.classList.toggle('drag-over', false);
        };

        this.#onSelectFile = async () => {
            try {
                const handle = await showOpenFilePicker({ multiple: true });
                for await (const value of handle.values()) {
                    console.log(value);
                }
            } catch (e) {
                console.error(e);
            }
        };
    }    

    connectedCallback() {
        this.addEventListener('dragleave', this.#onDragLeave, false);
        this.addEventListener('drop', this.#onDrop, false);
        this.addEventListener('dragover', this.#onDragOver, false);

        // We need to read attributes here, when they are set.
        this.#supportsOpenDialog = this.hasAttribute('supports-open-dialog');
        this.#parentId = +this.getAttribute('parent-id');
        
        this.#render();
    }

    disconnectedCallback() {
        this.removeEventListener('dragleave', this.#onDragLeave);
        this.removeEventListener('drop', this.#onDrop);
        this.removeEventListener('dragover', this.#onDragOver);

        this.#removeSelectFileEvent();
    }

    #addSelectFileEvent() {
        const select = this.shadowRoot.querySelector('.select-file');
        if (select) {
            select.addEventListener('click', this.#onSelectFile, false);
        }
    }

    #removeSelectFileEvent() {
        const select = this.shadowRoot.querySelector('.select-file');
        if (select) {
            select.removeEventListener('click', this.#onSelectFile);
        }
    }

    #render() {
        this.#removeSelectFileEvent();
        this.shadowRoot.replaceChildren(createTemplate(this.#supportsOpenDialog));
        this.#addSelectFileEvent();
    }


    async #readFileSystemHandlers(item) {
        const handle = await item.getAsFileSystemHandle();
    }

    async #readFileSystemEntry(item) {
        const entry = item.webkitGetAsEntry();
        await readEntry(this.#parentId, entry);
    }
}

async function readEntry(parentId, entry) {
    if (entry.isFile) {
        const file = await getFile(entry);
        await uploadFile(parentId, file);
    }
    if (entry.isDirectory) {
        const dirEntries = await readDirectoryEntriesAsync(entry);
        for (const entry of dirEntries) {
            await readEntry(entry);
        }
    }
}

function getFile(entry) {
    return new Promise((resolve,reject) => {
        entry.file(file => {
            resolve(file);
        }, err => reject(err))
    });
}

function readDirectoryEntriesAsync(entry) {
    const reader = entry.createReader();
    return new Promise((resolve,reject) => {
        reader.readEntries(entries => {
            resolve(entries);
        }, err => reject(err))
    });
}


async function uploadFile(parentId, file) {
    const token = JSON.parse(sessionStorage['app_user_context_key']).access_token.token;
    const headers = new Headers();
    headers.append('Authorization', `Bearer ${token}`);
    headers.append('X-File-Name', file.name);
    const request = new Request(`/api/files/upload_file/${parentId}`, {
        method: 'PUT',
        headers,
        body: file
    });
    
    const response = await fetch(request);
    const body = await response.json();
    return body.id;
}

function createFolder(parentId, folderName) {

}

const fileUpload = customElements.get('file-upload');
if (!fileUpload) {
    customElements.define('file-upload', FileUpload);
}
