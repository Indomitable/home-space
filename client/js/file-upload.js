function createTemplate() {
    const template = document.createElement('template');
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
    </style>
    <div class="drop-hint">
        <span>Drag and drop file(s) or folder(s) you want to upload</span>
    </div>
  `;

    return template.content.cloneNode(true);
}

class FileUpload extends HTMLElement {
    #onDragOver;
    #onDragLeave;
    #onDrop;

    constructor() {
        // Always call super first in constructor
        super();
        this.attachShadow({ mode: 'open' });
        this.shadowRoot.append(createTemplate());

        this.#onDragOver = (event) => {
            if (event.dataTransfer && event.dataTransfer.items.length > 0) {
                const item = event.dataTransfer.items[0];
                if (item.kind == 'file') {
                    this.classList.toggle('drag-over', true);
                    event.preventDefault();
                }
            }
        };
        this.#onDragLeave = () => {
            this.classList.toggle('drag-over', false);
        };
        this.#onDrop = (event) => {
            event.preventDefault();
            if (event.dataTransfer
                && event.dataTransfer.items.length > 0
                && event.dataTransfer.files.length > 0) {
                const item = event.dataTransfer.items[0];
                if (item.kind == 'file') {
                    const file = item.getAsFile();
                    if (file) {
                        console.log(file.name);
                    }
                }
            }
            this.classList.toggle('drag-over', false);
        };
    }

    connectedCallback() {
        this.addEventListener('dragleave', this.#onDragLeave, false);
        this.addEventListener('drop', this.#onDrop, false);
        this.addEventListener('dragover', this.#onDragOver, false);
    }

    disconnectedCallback() {
        this.removeEventListener('dragleave', this.#onDragLeave);
        this.removeEventListener('drop', this.#onDrop);
        this.removeEventListener('dragover', this.#onDragOver);
    }
}

const fileUpload = customElements.get('file-upload');
if (!fileUpload) {
    customElements.define('file-upload', FileUpload);
}
