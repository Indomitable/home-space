export async function uploadDataTransferItems(parentId, dataTransferItems) {
    const entries = dataTransferItems.map(i => i.webkitGetAsEntry());
    for (const entry of entries) {
        await uploadFileSystemEntry(parentId, entry);
    }
}

async function uploadFileSystemEntry(parentId, entry) {
    if (entry.isFile) {
        const file = await getFile(entry);
        // callback({ ftype: 1, name: entry.name });
        await uploadFile(parentId, file);
    }
    if (entry.isDirectory) {
        const dirEntries = await readDirectoryEntriesAsync(entry);
        // callback({ ftype: 0, name: entry.name });
        const parent_id = await createFolder(parentId, entry.name);
        for (const entry of dirEntries) {
            await uploadFileSystemEntry(parent_id, entry);
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


export async function uploadFile(parentId, file) {
    try {
        const token = JSON.parse(sessionStorage['app_user_context_key']).access_token.token;
        const headers = new Headers();
        headers.append('Authorization', `Bearer ${token}`);
        const formData = new FormData();
        formData.append("parent_id", parentId);
        formData.append("file", file, file.name);
        const request = new Request(`/api/files/upload_file`, {
            method: 'PUT',
            headers,
            body: formData
        });
        
        const response = await fetch(request);
        const body = await response.json();
        return body.id;
    } catch (e) {
        console.error(e);
        throw Error(e);
    }
}

async function createFolder(parentId, name) {
    const token = JSON.parse(sessionStorage['app_user_context_key']).access_token.token;
    const headers = new Headers();
    headers.append('Authorization', `Bearer ${token}`);
    headers.append('Content-Type', 'application/json');
    const request = new Request(`/api/files/create_folder`, {
        method: 'PUT',
        headers,
        body: JSON.stringify({parent_id: parentId, name})
    });
    
    const response = await fetch(request);
    const body = await response.json();
    return body.id;
}
