import { decode } from "@msgpack/msgpack";

export class Client {
    url: string;
    username: string;
    socket?: WebSocket;

    constructor(url: string, username: string) {
        this.url = url;
        this.username = username;
    }

    connect() {
        this.socket = new WebSocket(this.url);
        this.socket.binaryType = "arraybuffer";

        this.socket.addEventListener('message', (event) => {
            const obj = decode(event.data);
            console.log('Message', obj);
        });

        this.socket.addEventListener('close', (event) => {
            console.log('Close', event);
        });

        this.socket.addEventListener('error', (event) => {
            console.log('Error', event);
        });

        this.socket.addEventListener('open', (event) => {
            console.log('Open', event);
            this.socket.send('Hello Server!');
        });

    }

}
