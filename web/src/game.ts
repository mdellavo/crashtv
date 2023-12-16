import {
  AnimationAction,
  AnimationClip,
  AnimationMixer,
  ClampToEdgeWrapping,
  CubeTextureLoader,
  DataTexture,
  DirectionalLight,
  FogExp2,
  Group,
  Mesh,
  MeshBasicMaterial,
  MeshStandardMaterial,
  Object3D,
  PerspectiveCamera,
  PlaneGeometry,
  Scene,
  Skeleton,
  sRGBEncoding,
  Texture,
  TorusGeometry,
  Vector3,
  WebGLRenderer,
  Event
} from 'three';

import { GLTF } from 'three/examples/jsm/loaders/GLTFLoader';
import {OrbitControls} from "three/examples/jsm/controls/OrbitControls";


import Stats from 'stats.js';

import { Client, GameArea, GameObject, StateUpdate, Pong, Notice, ObjectType, ElevationMap, TerrainMap, ImageMap } from './api';

import SkyTop from './textures/sky/top.jpg';
import SkyBottom from './textures/sky/bottom.jpg';
import SkyFront from './textures/sky/front.jpg';
import SkyBack from './textures/sky/back.jpg';
import SkyRight from './textures/sky/right.jpg';
import SkyLeft from './textures/sky/left.jpg';


enum AnimationState {
  Idle,
  Running,
  Flying,
}

class ObjectModel {
  mesh: Object3D|Group;
  animationMixer: AnimationMixer;
  animationState: AnimationState;
  currentAnimation?: AnimationAction;

  constructor(mesh: Object3D|Group) {
    this.mesh = mesh;
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

      case AnimationState.Flying: {
        name = "Fast_Flying";
        break;
      }
    }

    this.animationState = state;

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


const buildItemMesh = (assets: Map<String, any>, item: GameObject): Mesh => {
  const material = new MeshStandardMaterial({color: 0xFFDF00});
  const mesh = new Mesh(ringGeom, material);
  mesh.position.x = SCALE * item.position.x;
  mesh.position.y = SCALE * item.position.y;
  mesh.position.z = SCALE * item.position.z;
  return mesh;
}

const buildActorMesh = (assets: Map<String, any>, item: GameObject): GLTF => {

  var keys = Array.from(assets.keys());
  var key = keys[Math.floor(Math.random() * keys.length)];

  let gltf = cloneGltf(assets.get(key));

  gltf.scene.scale.set(15, 15, 15);
  gltf.scene.rotation.y = Math.PI;
  gltf.scene.position.x = SCALE * item.position.x;
  gltf.scene.position.y = SCALE * item.position.y;
  gltf.scene.position.z = SCALE * item.position.z;

  return gltf;
}

const buildPlayerMesh = (assets: Map<String, any>, item: GameObject): GLTF => {
  const gltf = cloneGltf(assets.get("Dino"));

  gltf.scene.scale.set(20, 20, 20);
  gltf.scene.rotation.y = Math.PI;
  gltf.scene.position.x = SCALE * item.position.x;
  gltf.scene.position.y = SCALE * item.position.y;
  gltf.scene.position.z = SCALE * item.position.z;

  return gltf;
}

export interface GameProps {
    onNotice(message: string): void;
    onClose(): void;
    onError(error: any): void;
}

const SCALE = 100.0;

export const gameMain = (username: string, props: GameProps, assets: Map<String, any>) => {

  const renderer = new WebGLRenderer({
    antialias: true,
    alpha: true,
    logarithmicDepthBuffer: true,
  });
  renderer.shadowMap.enabled = true;
  renderer.outputEncoding = sRGBEncoding;
  renderer.setSize(window.innerWidth, window.innerHeight);
  renderer.setPixelRatio(window.devicePixelRatio);
  renderer.setClearColor(0x000000, 1);

  document.body.appendChild(renderer.domElement);

  const scene = new Scene();
  scene.fog = new FogExp2(0xeeeeee, 0.00001);

  const dirLight = new DirectionalLight(0xffffff, 1);
  dirLight.castShadow = true;
  dirLight.position.set(0, 1, 0).normalize();
  scene.add(dirLight);

  scene.background = new CubeTextureLoader().load([
    SkyRight, SkyLeft,
    SkyTop, SkyBottom,
    SkyFront, SkyBack,
  ]);

  var elevationMapMesh: Mesh;
  var terrainMapMesh: Mesh;
  var terrainTexture: Texture;
  var elevationTexture: Texture;

  const renderElevationTextureFromImageMap = (map: ImageMap) : Texture => {
    const data = new Uint8Array(4 * map.width * map.height);
    const size = map.width * map.height;

    var idx = 0;
    for (let i = 0; i<size; i ++) {
	    const stride = i * 4;
      const value = Math.ceil(map.data[idx++] * 255);
      data[stride] = value;
	    data[stride + 1] = value;
	    data[stride + 2] = value;
	    data[stride + 3] = 255;
    }

    const texture = new DataTexture(data, map.width, map.height);
    return texture;
  };

  const TerrainType = {
    Bare: 0x0,
    Beach: 0x1,
    Grassland: 0x2,
    Ocean: 0x3,
    Scorched: 0x4,
    Shrubland: 0x5,
    Snow: 0x6,
    SubtropicalDesert: 0x7,
    Taiga: 0x8,
    TemperateDeciduousForest: 0x9,
    TemperateDesert: 0xa,
    TemperateRainForest: 0xb,
    TropicalRainForest: 0xc,
    TropicalSeasonalForest: 0xd,
    Tundra: 0xe,
  } as {[k: string]: number};

  const TerrainColors = {
    Ocean: [0, 153, 152],
    Beach: [153, 255, 255],
    Bare: [128, 128, 128],
    Grassland: [102, 255, 102],
    Scorched: [192, 192, 192],
    Shrubland: [204, 204, 0],
    Snow: [255, 255, 255],
    SubtropicalDesert: [255, 204, 153],
    Taiga: [0, 204, 102],
    TemperateDeciduousForest: [102, 204, 0],
    TemperateDesert: [255, 153, 51],
    TemperateRainForest: [0, 204, 0],
    TropicalRainForest: [0, 255, 0],
    TropicalSeasonalForest: [51, 255, 51],
    Tundra: [204, 229, 255],
  } as {[k: string]: number[]};

  const renderTerrainTextureFromImageMap = (map: ImageMap) : Texture => {
    const data = new Uint8Array(4 * map.width * map.height);
    const size = map.width * map.height;

    const terrainColorMap = {} as {[k: number]: number[]};
    for (let terrainKey in TerrainType) {
      let terrainValue = TerrainType[terrainKey];
      let terrainColor = TerrainColors[terrainKey];
      terrainColorMap[terrainValue] = terrainColor;
    }

    var idx = 0;
    for (let i = 0; i<size; i ++) {
	    const stride = i * 4;

      const terrainValue = map.data[idx++];
      const terrainColor = terrainColorMap[terrainValue];

      data[stride] = terrainColor[0];
	    data[stride + 1] = terrainColor[1];
	    data[stride + 2] = terrainColor[2];
	    data[stride + 3] = 255;
    }

    const texture = new DataTexture(data, map.width, map.height);
    return texture;
  };

  const camera = new PerspectiveCamera(90, window.innerWidth / window.innerHeight, .01, 1000000);

  const controls = new OrbitControls(camera, renderer.domElement);
  controls.listenToKeyEvents(window);
  controls.addEventListener("start", (e: Event) => {
    console.log("start", e);
  });

  controls.addEventListener("change", (e: Event) => {
    console.log("change", e);
  });

  controls.addEventListener("end", (e: Event) => {
    console.log("end", e);
  });

  camera.position.z = -1000;
  camera.position.y = 150;
  camera.position.x = 0;
  camera.lookAt(new Vector3(0, 0, 0));

  controls.update();

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
  //window.addEventListener('keydown', onKeyDown);

  const onKeyUp = (e: KeyboardEvent) => {
    e.preventDefault();
    delete keyMap[e.key];
    if(!keyMap) {
      window.clearInterval(keyInterval);
    }
  };
  //window.addEventListener('keyup', onKeyUp);

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

          const isMoving = obj.velocity.magnitude() > 0;

          if (isMoving && model.animationState !== AnimationState.Running) {
            model.setAnimationState(AnimationState.Running);
          } else if (!isMoving && model.animationState !== AnimationState.Idle) {
            model.setAnimationState(AnimationState.Idle);
          }

          obj.position.x += (obj.velocity.x * delta);
          obj.position.y += (obj.velocity.y * delta);
          obj.position.z += (obj.velocity.z * delta);

          model.mesh.position.x = SCALE * obj.position.x;
          model.mesh.position.y = SCALE * obj.position.y;
          model.mesh.position.z = SCALE * obj.position.z;

          if (isMoving) {
            let dir = new Vector3(0, 0, 0);
            dir.add(new Vector3(obj.position.x, obj.position.y, obj.position.z));
            dir.add(new Vector3(obj.velocity.x, obj.velocity.y, obj.velocity.z));
            model.mesh.lookAt(dir);
          }
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
    "ElevationMap": (map: ElevationMap) => {
      elevationTexture = renderElevationTextureFromImageMap(map);
      elevationTexture.needsUpdate = true;
      elevationTexture.generateMipmaps = true;

      const elevationMapGeom = new PlaneGeometry(10, 10);
      const elevationMapMaterial = new MeshBasicMaterial({
        map: elevationTexture,
        depthTest: false,
      });
      elevationMapMesh = new Mesh(elevationMapGeom, elevationMapMaterial);
      elevationMapMesh.position.x = 0;
      elevationMapMesh.position.y = 0;
      elevationMapMesh.position.z = -5;
      scene.add(elevationMapMesh);

    },
    "TerrainMap": (map: TerrainMap) => {
      terrainTexture = renderTerrainTextureFromImageMap(map);
      terrainTexture.needsUpdate = true;
      terrainTexture.generateMipmaps = true;
      terrainTexture.wrapS = ClampToEdgeWrapping;
      terrainTexture.wrapT = ClampToEdgeWrapping;

      const terrainMapGeom = new PlaneGeometry(10, 10);
      const terrainMapMaterial = new MeshBasicMaterial({
        map: terrainTexture,
        depthTest: false,
      });
      terrainMapMesh = new Mesh(terrainMapGeom, terrainMapMaterial);
      terrainMapMesh.position.x = 0;
      terrainMapMesh.position.y = 0;
      terrainMapMesh.position.z = -5;
      scene.add(terrainMapMesh);

    },
    "StateUpdate": (state: StateUpdate) => {
      if (!state.incremental) {
        objectsGroup.clear();
        objectMap.clear();

        const planeGeometry = new PlaneGeometry(SCALE * state.areaSize, SCALE * state.areaSize, 1000, 1000);
        const planeMaterial = new MeshStandardMaterial({
          map: terrainTexture,
          displacementMap: elevationTexture,
          displacementScale: 2500,
          displacementBias: -500,  // FIXME
          //wireframe: true,
        });
        const plane = new Mesh(planeGeometry, planeMaterial);
        plane.position.y = -300;
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
        model.mesh.position.x = SCALE * obj.position.x;
        model.mesh.position.y = 10 * obj.position.y; // FIXME
        model.mesh.position.z = SCALE * obj.position.z;
        // console.log(obj.position);
      });

      const yourClient = objectMap.get(state.yourClientId);

      controls.update();

      elevationMapMesh.position.x = yourClient.mesh.position.x;
      elevationMapMesh.position.y = yourClient.mesh.position.y;
      elevationMapMesh.position.z = yourClient.mesh.position.z;

      terrainMapMesh.position.x = yourClient.mesh.position.x;
      terrainMapMesh.position.y = yourClient.mesh.position.y;
      terrainMapMesh.position.z = yourClient.mesh.position.z;

    },
  });

};
