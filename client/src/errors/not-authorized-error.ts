export class NotAuthorizedError extends Error {
    constructor(message: string ) {
        super();
        this.message = message;
        this.name = 'Not Authorized';
    }
}