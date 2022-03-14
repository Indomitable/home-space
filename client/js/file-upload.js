export async function uploadFileSystemEntry(parentId, entry) {
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
