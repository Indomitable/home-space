import Foundation
import Photos
import UniformTypeIdentifiers

/// The handle of the photo library source, which has only one place to look.
let photoLibraryHandle = "photo-library"

private func failed(_ detail: String) -> CoreError {
    .Failed(code: "local", detail: detail)
}

/// The phone's files, for auto-upload. A folder is a bookmark of what the user picked. iOS reports
/// no changes to one, so it is listed again on each pass. The photo library has no files, so an
/// asset is exported before it is read.
///
/// Rust calls these on a background thread, and they may block.
final class IOSSources: LocalSources, @unchecked Sendable {
    private let lock = NSLock()
    /// The asset exported last, which the reads of one upload share.
    private var exported: (key: String, url: URL)?
    /// What an asset's resources said, as long as the asset doesn't change: asking takes long
    /// enough that a large library would make every pass slow.
    private var known: [String: (Date?, LocalItem)] = [:]

    func list(kind: SourceKind, handle: String) throws -> [LocalItem] {
        switch kind {
        case .folder: try listFolder(handle)
        case .photos: try listPhotos()
        }
    }

    func stat(kind: SourceKind, handle: String, key: String) throws -> LocalItem? {
        switch kind {
        case .folder: try statFile(handle, key)
        case .photos: try statAsset(key)
        }
    }

    func read(kind: SourceKind, handle: String, key: String, offset: UInt64, len: UInt64) throws -> Data {
        switch kind {
        case .folder:
            return try withFolder(handle) { root in
                try readRange(of: root.appendingPathComponent(key), offset: offset, len: len)
            }
        case .photos:
            return try readRange(of: try export(key), offset: offset, len: len)
        }
    }

    // MARK: Folders. The key is the path below the folder.

    private func withFolder<T>(_ handle: String, _ work: (URL) throws -> T) throws -> T {
        guard let bookmark = Data(base64Encoded: handle) else {
            throw failed("The folder can't be found. Add it again.")
        }
        var stale = false
        let root: URL
        do {
            root = try URL(resolvingBookmarkData: bookmark, bookmarkDataIsStale: &stale)
        } catch {
            throw failed("The folder can't be found. Add it again.")
        }
        guard root.startAccessingSecurityScopedResource() else {
            throw failed("Home Space no longer has access to this folder. Add it again.")
        }
        defer { root.stopAccessingSecurityScopedResource() }
        return try work(root)
    }

    private func listFolder(_ handle: String) throws -> [LocalItem] {
        try withFolder(handle) { root in
            let keys: [URLResourceKey] = [.isRegularFileKey, .fileSizeKey, .contentModificationDateKey]
            guard let walk = FileManager.default.enumerator(
                at: root,
                includingPropertiesForKeys: keys,
                options: [.skipsHiddenFiles]
            ) else {
                throw failed("The folder can't be read.")
            }
            var items: [LocalItem] = []
            let base = root.standardizedFileURL.path
            for case let url as URL in walk {
                guard let item = fileItem(url, base: base) else { continue }
                items.append(item)
            }
            return items
        }
    }

    private func statFile(_ handle: String, _ key: String) throws -> LocalItem? {
        try withFolder(handle) { root in
            fileItem(root.appendingPathComponent(key), base: root.standardizedFileURL.path)
        }
    }

    private func fileItem(_ url: URL, base: String) -> LocalItem? {
        let keys: Set<URLResourceKey> = [.isRegularFileKey, .fileSizeKey, .contentModificationDateKey]
        guard let values = try? url.resourceValues(forKeys: keys), values.isRegularFile == true,
            let size = values.fileSize, let modified = values.contentModificationDate
        else { return nil }
        let path = url.standardizedFileURL.path
        guard path.hasPrefix(base + "/") else { return nil }
        let key = String(path.dropFirst(base.count + 1))
        let dir = (key as NSString).deletingLastPathComponent
        return LocalItem(
            key: key,
            relativeDir: dir,
            name: url.lastPathComponent,
            size: UInt64(size),
            modifiedAt: modified,
            mime: UTType(filenameExtension: url.pathExtension)?.preferredMIMEType
        )
    }

    private func readRange(of url: URL, offset: UInt64, len: UInt64) throws -> Data {
        let file = try FileHandle(forReadingFrom: url)
        defer { try? file.close() }
        try file.seek(toOffset: offset)
        return try file.read(upToCount: Int(len)) ?? Data()
    }

    // MARK: Photo library. The key is the asset's local identifier.

    private func listPhotos() throws -> [LocalItem] {
        let status = PHPhotoLibrary.authorizationStatus(for: .readWrite)
        guard status == .authorized || status == .limited else {
            throw failed("Home Space no longer has access to your photos.")
        }
        var items: [LocalItem] = []
        PHAsset.fetchAssets(with: nil).enumerateObjects { asset, _, _ in
            if let item = self.assetItem(asset) {
                items.append(item)
            }
        }
        return items
    }

    private func statAsset(_ key: String) throws -> LocalItem? {
        guard let asset = PHAsset.fetchAssets(withLocalIdentifiers: [key], options: nil).firstObject else {
            return nil
        }
        return assetItem(asset)
    }

    private func assetItem(_ asset: PHAsset) -> LocalItem? {
        let modified = asset.modificationDate ?? asset.creationDate
        lock.lock()
        if let (date, item) = known[asset.localIdentifier], date == modified {
            lock.unlock()
            return item
        }
        lock.unlock()
        guard let modified, let resource = primaryResource(of: asset) else { return nil }
        // PhotoKit has no public size. This key is the usual way, and without it the file has to
        // be exported, which is the only way to get the size that will be sent anyway.
        let size = (resource.value(forKey: "fileSize") as? NSNumber)?.uint64Value
            ?? (try? exportSize(asset.localIdentifier))
        guard let size else { return nil }
        let item = LocalItem(
            key: asset.localIdentifier,
            relativeDir: "",
            name: resource.originalFilename,
            size: size,
            modifiedAt: modified,
            mime: UTType(resource.uniformTypeIdentifier)?.preferredMIMEType
        )
        lock.lock()
        known[asset.localIdentifier] = (asset.modificationDate ?? asset.creationDate, item)
        lock.unlock()
        return item
    }

    /// The photo or video itself: the edited one if there is one, or the original.
    private func primaryResource(of asset: PHAsset) -> PHAssetResource? {
        let resources = PHAssetResource.assetResources(for: asset)
        let wanted: [PHAssetResourceType] =
            asset.mediaType == .video ? [.fullSizeVideo, .video] : [.fullSizePhoto, .photo]
        for type in wanted {
            if let found = resources.first(where: { $0.type == type }) {
                return found
            }
        }
        return nil
    }

    private func exportSize(_ key: String) throws -> UInt64 {
        let attributes = try FileManager.default.attributesOfItem(atPath: try export(key).path)
        return (attributes[.size] as? NSNumber)?.uint64Value ?? 0
    }

    /// Writes the asset to a temporary file, keeping only the last one.
    private func export(_ key: String) throws -> URL {
        lock.lock()
        defer { lock.unlock() }
        if let exported, exported.key == key {
            return exported.url
        }
        if let exported {
            try? FileManager.default.removeItem(at: exported.url)
        }
        exported = nil
        guard let asset = PHAsset.fetchAssets(withLocalIdentifiers: [key], options: nil).firstObject,
            let resource = primaryResource(of: asset)
        else {
            throw failed("The photo is no longer in the library.")
        }
        let url = FileManager.default.temporaryDirectory
            .appendingPathComponent("autoupload-\(UUID().uuidString)")
        let options = PHAssetResourceRequestOptions()
        // It may have to come from iCloud. A pass only runs on a network the settings allow.
        options.isNetworkAccessAllowed = true
        let done = DispatchSemaphore(value: 0)
        var failure: Error?
        PHAssetResourceManager.default().writeData(for: resource, toFile: url, options: options) {
            failure = $0
            done.signal()
        }
        done.wait()
        if let failure {
            throw failed("The photo can't be read: \(failure.localizedDescription)")
        }
        exported = (key, url)
        return url
    }
}
