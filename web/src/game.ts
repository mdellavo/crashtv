import {
  AmbientLight,
  AnimationClip,
  AnimationMixer,
  CubeTextureLoader,
  Scene,
  Group,
  PerspectiveCamera,
  WebGLRenderer,
  MeshStandardMaterial,
  MeshBasicMaterial,
  Mesh,
  Vector3,
  Skeleton,
  DoubleSide,
  PlaneGeometry,
  HemisphereLight,
  TextureLoader,
  RepeatWrapping,
  TorusGeometry,
  AnimationAction,
} from 'three';

import { GLTF } from 'three/examples/jsm/loaders/GLTFLoader';

import Stats from 'stats.js';

import { Client, GameArea, GameObject, StateUpdate, Pong, Notice, ObjectType } from './api';

import SkyTop from './textures/sky/top.jpg';
import SkyBottom from './textures/sky/bottom.jpg';
import SkyFront from './textures/sky/front.jpg';
import SkyBack from './textures/sky/back.jpg';
import SkyRight from './textures/sky/right.jpg';
import SkyLeft from './textures/sky/left.jpg';

import Grass from './textures/grass/grass03.png';


enum AnimationState {
  Idle,
  Running,
  Flying,
}

class ObjectModel {
  mesh: Group;
  animationMixer: AnimationMixer;
  animationState: AnimationState;
  currentAnimation?: AnimationAction;

  constructor(mesh: Group) {
    this.animationMixer = new AnimationMixer(mesh);
    this.animationState = AnimationState.Idle;
  }

  get animations() : AnimationClip[] {
    return [];
  }

  playAnimationClip(name: string) {
    var clip = AnimationClip.findByName(this.animations, name);
    if (clip) {

      if (this.currentAnimation) {
        this.currentAnimation.stop();
        this.currentAnimation = null;
      }

      this.currentAnimation = this.animationMixer.clipAction(clip);
      this.currentAnimation.play();
      return true;
    }
    return false;
  }

  setAnimationState(state: AnimationState) {
    var name;

    switch (state) {
      case AnimationState.Idle: {
        name = "Idle";
        break;
      }

      case AnimationState.Running: {
        name = "Run";
        break;
      }
    }

    return this.playAnimationClip(name);
  }
}

class GLTFObjectModel extends ObjectModel {
  gltf: GLTF;

  constructor(gltf: GLTF) {
    super(gltf.scene);
    this.gltf = gltf;
  }

  get animations() : AnimationClip[] {
    return this.gltf.animations || [];
  }

}


const cloneGltf = (gltf: GLTF): GLTF => {

  const clone = {
    animations: gltf.animations,
    scene: gltf.scene.clone(true)
  };

  const skinnedMeshes = {} as {[key: string]: any};

  gltf.scene.traverse((node: any) => {
    if (node.isSkinnedMesh) {
      skinnedMeshes[node.name] = node;
    }
  });

  const cloneBones = {} as {[key: string]: any};
  const cloneSkinnedMeshes = {} as {[key: string]: any};

  clone.scene.traverse((node: any) => {
    if (node.isBone) {
      cloneBones[node.name] = node;
    }

    if (node.isSkinnedMesh) {
      cloneSkinnedMeshes[node.name] = node;
    }
  });

  for (let name in skinnedMeshes) {
    const skinnedMesh = skinnedMeshes[name];
    const skeleton = skinnedMesh.skeleton;
    const cloneSkinnedMesh = cloneSkinnedMeshes[name];

    const orderedCloneBones = [];

    for (let i = 0; i < skeleton.bones.length; ++i) {
      const cloneBone = cloneBones[skeleton.bones[i].name];
      orderedCloneBones.push(cloneBone);
    }

    cloneSkinnedMesh.bind(
        new Skeleton(orderedCloneBones, skeleton.boneInverses),
        cloneSkinnedMesh.matrixWorld);
  }

  clone.scene.traverse((node: any) => {
    if (node.material) {
      node.material.metalness = 0;
    }
  });

  return clone as GLTF;
}

const ringGeom = new TorusGeometry(8, 3, 16, 100);


const buildItemMesh = (assets: Map<String, any>, item: GameObject): Group => {
  const material = new MeshStandardMaterial({color: 0xFFDF00});
  const mesh = new Mesh(ringGeom, material);
  mesh.position.x = item.position.x;
  mesh.position.y = item.position.y;
  mesh.position.z = item.position.z;

  const group = new Group();
  group.add(mesh);

  return group;
}

const buildActorMesh = (assets: Map<String, any>, item: GameObject): GLTF => {

  var keys = Array.from(assets.keys());
  var key = keys[Math.floor(Math.random() * keys.length)];

  let gltf = cloneGltf(assets.get(key));

  gltf.scene.scale.set(15, 15, 15);
  gltf.scene.rotation.y = Math.PI;
  gltf.scene.position.x = item.position.x;
  gltf.scene.position.y = item.position.y;
  gltf.scene.position.z = item.position.z;

  return gltf;
}

const buildPlayerMesh = (assets: Map<String, any>, item: GameObject): GLTF => {
  const gltf = cloneGltf(assets.get("Dino"));

  gltf.scene.scale.set(20, 20, 20);
  gltf.scene.rotation.y = Math.PI;
  gltf.scene.position.x = item.position.x;
  gltf.scene.position.y = item.position.y;
  gltf.scene.position.z = item.position.z;

  return gltf;
}

export interface GameProps {
    onNotice(message: string): void;
    onClose(): void;
    onError(error: any): void;
}

export const gameMain = (username: string, props: GameProps, assets: Map<String, any>) => {

  const renderer = new WebGLRenderer({
    antialias: true,
    alpha: true,
  });

  renderer.setSize(window.innerWidth, window.innerHeight);
  renderer.setPixelRatio(window.devicePixelRatio);
  renderer.setClearColor(0x000000, 1);

  document.body.appendChild(renderer.domElement);

  const scene = new Scene();

  var hemiLight = new HemisphereLight( 0xffffff, 0xffffff );
  hemiLight.position.set(0, 100, 0);
  scene.add(hemiLight);

  const light = new AmbientLight(0xffffff);
  scene.add(light);

  scene.background = new CubeTextureLoader().load([
    SkyRight, SkyLeft,
    SkyTop, SkyBottom,
    SkyFront, SkyBack,
  ]);

  const camera = new PerspectiveCamera(90, window.innerWidth / window.innerHeight, .1, 2500);
  camera.position.z = -250;
  camera.position.y = 250;
  camera.position.x = 0;
  camera.lookAt(new Vector3(0, 0, 0));

  const objectsGroup = new Group();
  scene.add(objectsGroup);

  const objectMap = new Map();

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

  var stats = new Stats();
  document.body.appendChild( stats.dom );

  let previousTimestamp : number = null;
  let tick = 0;
  const animate = (elapsed: number) => {
    stats.begin();

    if (!previousTimestamp) {
      previousTimestamp = elapsed;
    }
    if (elapsed !== previousTimestamp) {
      const delta = (elapsed - previousTimestamp) / 1000.0;

      area.objects.forEach((obj: GameObject) => {

        var model = objectMap.get(obj.objectId);
        model.animationMixer.update(delta);

        if (obj.objectType.toString() !== "Item") {
          obj.velocity.x += (obj.acceleration.x * delta);
          obj.velocity.y += (obj.acceleration.y * delta);
          obj.velocity.z += (obj.acceleration.z * delta);

          obj.position.x += (obj.velocity.x * delta);
          obj.position.y += (obj.velocity.y * delta);
          obj.position.z += (obj.velocity.z * delta);

          model.mesh.position.x = obj.position.x;
          model.mesh.position.y = obj.position.y;
          model.mesh.position.z = obj.position.z;

          let dir = new Vector3(0, 0, 0);
          dir.add(new Vector3(obj.position.x, obj.position.y, obj.position.z));
          dir.add(new Vector3(obj.velocity.x, obj.velocity.y, obj.velocity.z));

          model.mesh.lookAt(dir);
        }

        if (obj.objectType.toString() === "Item") {
          model.mesh.rotation.y = tick * .00001;
        }

        tick += 1;
      });
    }

	  renderer.render(scene, camera);
    stats.end();

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

        const planeGeometry = new PlaneGeometry(2.0 * state.areaSize, 2.0 * state.areaSize);

        const texture = new TextureLoader().load(Grass);
        texture.wrapS = RepeatWrapping;
        texture.wrapT = RepeatWrapping;

        const planeMaterial = new MeshBasicMaterial({map: texture, side: DoubleSide});
        const plane = new Mesh( planeGeometry, planeMaterial );
        plane.rotateX(-Math.PI/2);
        scene.add(plane);
      }

      var [added, removed, updated] = area.update(state);

      added.forEach((obj: GameObject) => {
        var model = objectMap.get(obj.objectId);

        if (!model) {

          if (obj.objectType.toString() === "Item") {
            obj.position.y += 25; // FIXME
            model = new ObjectModel(buildItemMesh(assets, obj));
          } else if (obj.objectType.toString() === "Actor") {
            model = new GLTFObjectModel(buildActorMesh(assets, obj));
          } else {
            model = new GLTFObjectModel(buildPlayerMesh(assets, obj));
          }
          objectMap.set(obj.objectId, model);

          model.setAnimationState(AnimationState.Idle);

          objectsGroup.add(model.mesh);
        }
      });

      if (removed) {
        removed.forEach((obj: GameObject) => {
          var model = objectMap.get(obj.objectId);
          objectsGroup.remove(model.mesh);
          objectMap.delete(obj.objectId);
        });
      }

      updated.forEach((obj: GameObject) => {
        var model = objectMap.get(obj.objectId);
        model.mesh.position.x = obj.position.x;
        model.mesh.position.y = obj.position.y;
        model.mesh.position.z = obj.position.z;
      });

      const yourClient = objectMap.get(state.yourClientId);
      camera.position.x = yourClient.mesh.position.x;
      camera.position.z = yourClient.mesh.position.z - 250;
      camera.lookAt(yourClient.mesh.position);

    },
  });

};
