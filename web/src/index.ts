import './style.css';

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

import { Client, GameArea, GameObject, StateUpdate, Pong, Notice, ErrorMessage } from './api';

const buildItemMesh = (item: GameObject) => {
    const geometry = new BoxGeometry(1, 1, 1);
    const material = new MeshBasicMaterial({color: 0x00ff00});
    const cube = new Mesh(geometry, material);
    cube.position.x = item.position.x;
    cube.position.y = item.position.y;
    cube.position.z = item.position.z;
    return cube;
}


const main = () => {
    const renderer = new WebGLRenderer();
    renderer.setSize(window.innerWidth, window.innerHeight);
    document.body.appendChild(renderer.domElement);

    const scene = new Scene();

    const camera = new PerspectiveCamera(75, window.innerWidth / window.innerHeight, 0.1, 1000);
    camera.position.z = 1000;
    camera.position.y = 100;
    camera.position.x = 500;
    camera.lookAt(new Vector3(0, 0, 0));

    const objectsGroup = new Group();
    const objectMap = new Map();
    scene.add(objectsGroup);

    const animate = () => {
	    requestAnimationFrame(animate);
	    renderer.render(scene, camera);
    }
    animate();

    const area = new GameArea();
    const client = new Client("ws://localhost:3030/ws", "user");
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
            var [added, removed] = area.update(state);

            added.forEach((obj: GameObject) => {
                var mesh = buildItemMesh(obj);
                objectMap.set(obj.objectId, mesh);
                objectsGroup.add(mesh);
            });

            removed.forEach((obj: GameObject) => {
                var mesh = objectMap.get(obj.objectId);
                objectsGroup.remove(mesh);
                objectMap.delete(obj.objectId);
            });

        },
    });

};
main();
