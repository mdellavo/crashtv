import { decode as msgpack_decode, encode as msgpack_encode } from "@msgpack/msgpack";

export enum ObjectType {
    ITEM,
    PLAYER,
    ENEMY,
}

export class Vec3 {
    x: number;
    y: number;
    z: number;

    constructor(x: number, y: number, z: number) {
        this.x = x;
        this.y = y;
        this.z = z;
    }

    static fromResponse(data: any) {
        return new Vec3(data[0], data[1], data[2]);
    }
}

export class GameObject {
    objectId: number;
    objectType: ObjectType;
    position: Vec3;
    velocity: Vec3;

    constructor(objectId: number, objectType: ObjectType, position: Vec3, velocity: Vec3) {
        this.objectId = objectId;
        this.objectType = objectType;
        this.position = position;
        this.velocity = velocity;
    }

    static fromResponse(data: any) {
        const objId = data[0];
        const objType = data[1] as ObjectType;
        const position = Vec3.fromResponse(data[2]);
        const velocity = Vec3.fromResponse(data[3]);
        return new GameObject(objId, objType, position, velocity)
    }
}

export class GameArea {
    areaSize: number;
    objects: Map<number, GameObject>;

    constructor() {
        this.areaSize = -1;
        this.objects = new Map();
    }

    update(state: StateUpdate) {
        this.areaSize = state.areaSize;

        var current = new Set(this.objects.keys());
        var updated = new Set<number>();

        for(var i=0; i<state.objects.length; i++) {
            var obj = state.objects[i];
            this.objects.set(obj.objectId, obj);
            updated.add(obj.objectId);
        }

        var addedIds = new Set(Array.from(updated).filter(x => !current.has(x)));
        var removedIds = new Set(Array.from(current).filter(x => !updated.has(x)));

        var added = new Set<GameObject>(Array.from(addedIds).map(x => this.objects.get(x)));
        var removed = new Set<GameObject>(Array.from(removedIds).map(x => this.objects.get(x)));

        removedIds.forEach((removedId) => {
            this.objects.delete(removedId);
        });

        return [added, removed];
    }
}

export class StateUpdate {
    areaSize: number;
    objects: GameObject[];

    constructor(areaSize: number, objects: GameObject[]) {
        this.areaSize = areaSize;
        this.objects = objects;
    }

    static fromResponse(data: any) {
        const objects = data[1].map(GameObject.fromResponse);
        return new StateUpdate(data[0], objects);
    }
}

export class Pong {
    timestamp: number;

    constructor(timestamp: number) {
        this.timestamp = timestamp;
    }

    static fromResponse(data: any) {
        return new Pong(data);
    }
}

export class Notice {
    message: string;

    constructor(message: string) {
        this.message = message;
    }

    static fromResponse(data: any) {
        return new Notice(data);
    }
}

export class ErrorMessage {
    code: number;
    message: string;
    constructor(code: number, message: string) {
        this.code = code;
        this.message = message;
    }

    static fromResponse(data: any) {
        return new ErrorMessage(data[0], data[1]);
    }
}

const decoders = {
    "StateUpdate": (data: any) => StateUpdate.fromResponse(data),
    "Pong": (data: any) => Pong.fromResponse(data),
    "Notice": (data: any) => Notice.fromResponse(data),
    "Error": (data: any) => ErrorMessage.fromResponse(data),

} as {[key: string]: any};


const decodeResponse = (data: {[key: string]: any}) => {
    const rv = {} as {[key: string]: any};
    for (let k in data) {
        const decoder = decoders[k];
        rv[k] = decoder(data[k]);
    }
    return rv;
};


export class Client {
    url: string;
    username: string;
    socket?: WebSocket;

    constructor(url: string, username: string) {
        this.url = url;
        this.username = username;
    }

    connect(eventHandler: any) {
        this.socket = new WebSocket(this.url);
        this.socket.binaryType = "arraybuffer";

        this.socket.addEventListener('message', (event) => {
            const rawResponse = msgpack_decode(event.data) as {string: any};
            const response = decodeResponse(rawResponse);

            console.log('Message', response);

            for (let k in response) {
                var handler = eventHandler[k];
                if (handler) {
                    handler(response[k]);
                }
            }
        });

        this.socket.addEventListener('close', (event) => {
            console.log('Close', event);
        });

        this.socket.addEventListener('error', (event) => {
            console.log('Error', event);
        });

        this.socket.addEventListener('open', (event) => {
            console.log('Open', event);
            this.sendHello(this.username);
            this.sendPing();
        });
    }

    send(msg: any) {
        console.log("Sending", msg);
        var data = msgpack_encode(msg);
        this.socket.send(data);
    }

    sendHello(username: string) {
        this.send({
            "Hello": username,
        });
    }

    sendPing() {
        this.send({
            "Ping": new Date().getTime()
        });
    }
}
