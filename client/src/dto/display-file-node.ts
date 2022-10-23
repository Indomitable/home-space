import type {FileNodeResponse} from "./file-node-response";

export interface DisplayFileNode extends FileNodeResponse {
    isFavorite: boolean;
}