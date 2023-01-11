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
import { OrbitControls } from 'three/examples/jsm/controls/OrbitControls';

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

const getRandomInt = (max: number) => {
  return Math.floor(Math.random() * max);
}

const main = () => {
  const renderer = new WebGLRenderer();
  renderer.setSize(window.innerWidth, window.innerHeight);
  renderer.setPixelRatio(window.devicePixelRatio);
  document.body.appendChild(renderer.domElement);

  const scene = new Scene();

  const camera = new PerspectiveCamera(75, window.innerWidth / window.innerHeight, 0.1, 10000);
  camera.position.z = 550;
  camera.position.y = 50;
  camera.position.x = 550;
  camera.lookAt(new Vector3(500, 0, 500));

  var lastCameraPosition = new Vector3(0, 0, 0);
  lastCameraPosition.copy(camera.position);

	const controls = new OrbitControls(camera, renderer.domElement);
	controls.listenToKeyEvents(window);
  controls.enableDamping = true;
	controls.dampingFactor = 0.05;
	controls.screenSpacePanning = false;
	controls.minDistance = 100;
	controls.maxDistance = 5000;
	controls.maxPolarAngle = Math.PI / 2;

  const objectsGroup = new Group();
  const objectMap = new Map();
  scene.add(objectsGroup);

	const onWindowResize = () => {
		camera.aspect = window.innerWidth / window.innerHeight;
		camera.updateProjectionMatrix();

		renderer.setSize( window.innerWidth, window.innerHeight );
	}
  window.addEventListener('resize', onWindowResize);

  const animate = () => {
	  requestAnimationFrame(animate);
    controls.update();
	  renderer.render(scene, camera);
  }
  animate();

  const area = new GameArea();

  const username = `user-${getRandomInt(100)}`;
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

    },
  });

  controls.addEventListener("change", (e) => {
    var delta = new Vector3(0, 0, 0);
    delta.copy(camera.position);
    delta.sub(lastCameraPosition);
    delta.normalize();
    lastCameraPosition.copy(camera.position);
    client.sendMove(delta.x, 0, delta.z);
  });

};
main();
