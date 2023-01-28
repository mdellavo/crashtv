import { decode as msgpack_decode, encode as msgpack_encode } from "@msgpack/msgpack";

export enum ObjectType {
  Item,
  Actor,
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
  alive: boolean;
  age: number;
  objectId: number;
  objectType: ObjectType;
  position: Vec3;
  velocity: Vec3;

  constructor(alive: boolean, age: number, objectId: number, objectType: ObjectType, position: Vec3, velocity: Vec3) {
    this.alive = alive;
    this.age = age;
    this.objectId = objectId;
    this.objectType = objectType;
    this.position = position;
    this.velocity = velocity;
  }

  static fromResponse(data: any) {
    const alive = data[0];
    const age = data[1]
    const objId = data[2];
    const objType = data[3] as ObjectType;
    const position = Vec3.fromResponse(data[4]);
    const velocity = Vec3.fromResponse(data[5]);
    return new GameObject(alive, age, objId, objType, position, velocity);
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

    if (!state.incremental) {
      var yourObj = this.objects.get(state.yourClientId);
      this.objects.clear();
      if (yourObj)
        this.objects.set(yourObj.objectId, yourObj);
    }

    var current = new Set<number>();
    var touched = new Set<number>();

    var deadIds = [];
    for(var i=0; i<state.objects.length; i++) {
      var obj = state.objects[i];
      if (obj.alive) {
        this.objects.set(obj.objectId, obj);
        touched.add(obj.objectId);
      } else {
        deadIds.push(obj.objectId);
      }
    }

    var addedIds = new Set(Array.from(touched).filter(x => !current.has(x)));
    var removedIds = new Set(Array.from(current).filter(x => !touched.has(x)));
    for (var i=0; i<deadIds.length; i++) {
      removedIds.add(deadIds[i]);
    }

    var updatedIds = new Set(Array.from(touched).filter(x => !addedIds.has(x) || !removedIds.has(x)))

    var added = new Set<GameObject>(Array.from(addedIds).map(x => this.objects.get(x)));
    var updated = new Set<GameObject>(Array.from(updatedIds).map(x => this.objects.get(x)));

    var removed = new Set<GameObject>(Array.from(removedIds).map(x => this.objects.get(x)));
    removedIds.forEach((removedId) => {
      this.objects.delete(removedId);
    });

    return [added, removed, updated];
  }
}

export class StateUpdate {
  yourClientId: number;
  areaSize: number;
  incremental: boolean;
  objects: GameObject[];

  constructor(yourClientId: number, areaSize: number, incremental: boolean, objects: GameObject[]) {
    this.yourClientId = yourClientId;
    this.areaSize = areaSize;
    this.incremental = incremental;
    this.objects = objects;
  }

  static fromResponse(data: any) {
    const objects = data[3].map(GameObject.fromResponse);
    return new StateUpdate(data[0], data[1], data[2], objects);
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

    const callHandler = (name: string, arg?: any) => {
      var handler = eventHandler[name];
      if (handler) {
        handler(arg);
      }
    };

    try {
      this.socket = new WebSocket(this.url);
    } catch(error) {
      callHandler('Error', error);
    }

    this.socket.binaryType = "arraybuffer";

    this.socket.addEventListener('message', (event) => {
      const rawResponse = msgpack_decode(event.data) as {string: any};
      const response = decodeResponse(rawResponse);

      for (let k in response) {
        callHandler(k, response[k]);
      }
    });

    this.socket.addEventListener('close', (event) => {
      console.log('Close', event);
      this.socket = undefined;
      callHandler('Close', event);
    });

    this.socket.addEventListener('error', (event) => {
      console.log('Error', event);
      callHandler('Error', event);
    });

    this.socket.addEventListener('open', (event) => {
      console.log('Open', event);
      callHandler('Open', event);
      this.sendHello(this.username);
    });
  }

  send(msg: any) {
    if (!this.socket) {
      return;
    }
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

  sendMove(x: number, y: number, z: number) {
    this.send({
      "Move": [x, y, z],
    });
  }
}
