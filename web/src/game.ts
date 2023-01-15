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
  const material = new MeshBasicMaterial({color: 0x0000ff});
  const cube = new Mesh(geometry, material);
  cube.position.x = item.position.x;
  cube.position.y = item.position.y;
  cube.position.z = item.position.z;
  return cube;
}


export const gameMain = (username: string) => {
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

  const onKeyPress = (e: KeyboardEvent) => {
    e.preventDefault();

    switch(e.key) {
        case 'w':
        case 'ArrowUp': {
          client.sendMove(0, 0, 1);
          camera.position.z += 1;
          break;
        }

        case 's':
        case 'ArrowDown': {
          client.sendMove(0, 0, -1);
          break;
        }

        case 'a':
        case 'ArrowLeft': {
          client.sendMove(1, 0, 0);
          break;
        }

        case 'd':
        case 'ArrowRight': {
          client.sendMove(-1, 0, 0);
          break;
        }
    }
  }
  window.addEventListener('keydown', onKeyPress);

  const animate = () => {
	  requestAnimationFrame(animate);
	  renderer.render(scene, camera);
  }
  animate();

  const area = new GameArea();

  const client = new Client("ws://localhost:3030/ws", username);
  client.connect({
    "Pong": (pong: Pong) => {
      const now = new Date().getTime();
      console.log("pong", now - pong.timestamp, "ms");
    },
    "Notice": (notice: Notice) => {
      console.log("notice from server:", notice.message);
    },
    "Error": (error: ErrorMessage) => {
      console.log(error);
    },
    "StateUpdate": (state: StateUpdate) => {
      var [added, removed, updated] = area.update(state);

      added.forEach((obj: GameObject) => {
        if (obj.objectType.toString() === "Item") {
          var mesh = buildItemMesh(obj);
        } else {
          var mesh = buildActorMesh(obj);
        }
        objectMap.set(obj.objectId, mesh);
        objectsGroup.add(mesh);
      });

      removed.forEach((obj: GameObject) => {
        var mesh = objectMap.get(obj.objectId);
        objectsGroup.remove(mesh);
        objectMap.delete(obj.objectId);
      });

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
