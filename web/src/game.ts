import {
  Scene,
  Group,
  PerspectiveCamera,
  WebGLRenderer,
  BoxGeometry,
  MeshBasicMaterial,
  Mesh,
  Vector3,
} from 'three';

import { Client, GameArea, GameObject, StateUpdate, Pong, Notice, ErrorMessage, ObjectType, Vec3 } from './api';

const buildItemMesh = (item: GameObject) => {
  const geometry = new BoxGeometry(1, 1, 1);
  const material = new MeshBasicMaterial({color: 0x00ff00});
  const cube = new Mesh(geometry, material);
  cube.position.x = item.position.x;
  cube.position.y = item.position.y;
  cube.position.z = item.position.z;
  return cube;
}

const buildActorMesh = (item: GameObject) => {
  const geometry = new BoxGeometry(5, 5, 5);
  const material = new MeshBasicMaterial({color: 0xff0000});
  const cube = new Mesh(geometry, material);
  cube.position.x = item.position.x;
  cube.position.y = item.position.y;
  cube.position.z = item.position.z;
  return cube;
}

const buildPlayerMesh = (item: GameObject) => {
  const geometry = new BoxGeometry(5, 5, 5);
  const material = new MeshBasicMaterial({color: 0x0000ff});
  const cube = new Mesh(geometry, material);
  cube.position.x = item.position.x;
  cube.position.y = item.position.y;
  cube.position.z = item.position.z;
  return cube;
}

export interface GameProps {
    onNotice(message: string): void;
    onClose(): void;
    onError(error: any): void;
}

export const gameMain = (username: string, props: GameProps) => {
  const renderer = new WebGLRenderer();
  renderer.setSize(window.innerWidth, window.innerHeight);
  renderer.setPixelRatio(window.devicePixelRatio);
  document.body.appendChild(renderer.domElement);

  const scene = new Scene();

  const camera = new PerspectiveCamera(90, window.innerWidth / window.innerHeight, 0.1, 10000);
  camera.position.z = -200;
  camera.position.y = 200;
  camera.position.x = 0;
  camera.lookAt(new Vector3(0, 0, 0));

  const objectsGroup = new Group();
  const objectMap = new Map();
  scene.add(objectsGroup);

	const onWindowResize = () => {
		camera.aspect = window.innerWidth / window.innerHeight;
		camera.updateProjectionMatrix();
		renderer.setSize(window.innerWidth, window.innerHeight);
	}
  window.addEventListener('resize', onWindowResize);

  const keyMap = {} as {[key: string]: boolean};

  const checkKeys = () => {
    var [x, y, z] = [0, 0, 0];
    if (keyMap['w'] || keyMap["ArrowUp"]) {
          z = 1;
    }
    if (keyMap['s'] || keyMap["ArrowDown"]) {
          z = -1;
    }
    if (keyMap['a'] || keyMap["ArrowLeft"]) {
          x = 1;
    }
    if (keyMap['d'] || keyMap["ArrowRight"]) {
          x = -1;
    }
    client.sendMove(x, y, z);
  }

  var keyInterval :number|undefined;
  const onKeyDown = (e: KeyboardEvent) => {
    e.preventDefault();
    keyMap[e.key] = true;
    if (!keyInterval) {
      keyInterval = window.setInterval(checkKeys, 1000/24);
    }
  };
  window.addEventListener('keydown', onKeyDown);

  const onKeyUp = (e: KeyboardEvent) => {
    e.preventDefault();
    delete keyMap[e.key];
    if(!keyMap) {
      window.clearInterval(keyInterval);
    }
  };
  window.addEventListener('keyup', onKeyUp);

  const area = new GameArea();

  let previousTimestamp : number = null;
  const animate = (elapsed: number) => {

    if (!previousTimestamp) {
      previousTimestamp = elapsed;
    }
    if (elapsed !== previousTimestamp) {
      const delta = (elapsed - previousTimestamp) / 1000.0;

      area.objects.forEach((obj: GameObject) => {
        obj.velocity.x += (obj.acceleration.x * delta);
        obj.velocity.y += (obj.acceleration.y * delta);
        obj.velocity.z += (obj.acceleration.z * delta);

        obj.position.x += (obj.velocity.x * delta);
        obj.position.y += (obj.velocity.y * delta);
        obj.position.z += (obj.velocity.z * delta);

        var mesh = objectMap.get(obj.objectId);
        mesh.position.x = obj.position.x;
        mesh.position.y = obj.position.y;
        mesh.position.z = obj.position.z;
      }) ;

    }

	  renderer.render(scene, camera);
	  requestAnimationFrame(animate);
    previousTimestamp = elapsed;
  }
	requestAnimationFrame(animate);

  var timer: any = undefined;
  const client = new Client("ws://localhost:3030/ws", username);
  const onInterval = () => {
    client.sendPing()
  };

  client.connect({
    "Open": () => {
      timer = window.setInterval(onInterval, 5000);
    },
    "Close": () => {
      if (timer) {
        window.clearInterval(timer);
      }
      props.onClose()
    },
    "Error": (e: any) => {
      if (timer) {
        window.clearInterval(timer);
      }
      props.onError(e);
    },
    "Pong": (pong: Pong) => {
      const now = new Date().getTime();
      console.log("pong", now - pong.timestamp, "ms");
    },
    "Notice": (notice: Notice) => {
      console.log("notice from server:", notice.message);
      props.onNotice(notice.message);
    },
    "StateUpdate": (state: StateUpdate) => {

      if (!state.incremental) {
        objectsGroup.clear();
        objectMap.clear();
      }

      var [added, removed, updated] = area.update(state);

      //console.log("update", "objectMap", objectMap.size, "group", objectsGroup.children.length, "added", added.size, "removed", removed.size, "updated", updated.size);

      added.forEach((obj: GameObject) => {
        var mesh = objectMap.get(obj.objectId);

        if (!mesh) {
          if (obj.objectType.toString() === "Item") {
            mesh = buildItemMesh(obj);
          } else if (obj.objectType.toString() === "Actor") {
            mesh = buildActorMesh(obj);
          } else {
            mesh = buildPlayerMesh(obj);
          }
          objectMap.set(obj.objectId, mesh);
          objectsGroup.add(mesh);
        }

      });

      if (removed) {
        removed.forEach((obj: GameObject) => {
          var mesh = objectMap.get(obj.objectId);
          objectsGroup.remove(mesh);
          objectMap.delete(obj.objectId);
        });
      }

      updated.forEach((obj: GameObject) => {
        var mesh = objectMap.get(obj.objectId);
        mesh.position.x = obj.position.x;
        mesh.position.y = obj.position.y;
        mesh.position.z = obj.position.z;
      });

      const yourClient = objectMap.get(state.yourClientId);
      camera.position.x = yourClient.position.x;
      camera.position.z = yourClient.position.z - 200;
      camera.lookAt(yourClient.position);

    },
  });

};
