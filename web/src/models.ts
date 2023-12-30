import { GLTFLoader } from "three/examples/jsm/loaders/GLTFLoader";

import Alien from "./models/Alien.gltf";
import Alpaking from "./models/Alpaking.gltf";
import AlpakingEvolved from "./models/Alpaking_Evolved.gltf";
import Armabee from "./models/Armabee.gltf";
import ArmabeeEvolved from "./models/Armabee_Evolved.gltf";
import Birb from "./models/Birb.gltf";
import BlueDemon from "./models/BlueDemon.gltf";
import Bunny from "./models/Bunny.gltf";
import Cactoro from "./models/Cactoro.gltf";
import Cat from "./models/Cat.gltf";
import Chicken from "./models/Chicken.gltf";
import Demon from "./models/Demon.gltf";
import Dino from "./models/Dino.gltf";
import Dog from "./models/Dog.gltf";
import Dragon from "./models/Dragon.gltf";
import DragonEvolved from "./models/Dragon_Evolved.gltf";
import Fish from "./models/Fish.gltf";
import Frog from "./models/Frog.gltf";
import Ghost from "./models/Ghost.gltf";
import GhostSkull from "./models/Ghost_Skull.gltf";
import Glub from "./models/Glub.gltf";
import GlubEvolved from "./models/Glub_Evolved.gltf";
import Goleling from "./models/Goleling.gltf";
import GolelingEvolved from "./models/Goleling_Evolved.gltf";
import GreenBlob from "./models/GreenBlob.gltf";
import GreenSpikyBlob from "./models/GreenSpikyBlob.gltf";
import Hywirl from "./models/Hywirl.gltf";
import Monkroose from "./models/Monkroose.gltf";
import Mushnub from "./models/Mushnub.gltf";
import MushnubEvolved from "./models/Mushnub_Evolved.gltf";
import MushroomKing from "./models/MushroomKing.gltf";
import Ninja from "./models/Ninja.gltf";
import Orc from "./models/Orc.gltf";
import OrcSkull from "./models/Orc_Skull.gltf";
import Pigeon from "./models/Pigeon.gltf";
import PinkBlob from "./models/PinkBlob.gltf";
import Squidle from "./models/Squidle.gltf";
import Tribal from "./models/Tribal.gltf";
import Wizard from "./models/Wizard.gltf";
import Yeti from "./models/Yeti.gltf";

const allModels = {
  Alien: Alien,
  Alpaking: Alpaking,
  AlpakingEvolved: AlpakingEvolved,
  Armabee: Armabee,
  ArmabeeEvolved: ArmabeeEvolved,
  Birb: Birb,
  BlueDemon: BlueDemon,
  Bunny: Bunny,
  Cactoro: Cactoro,
  Cat: Cat,
  Chicken: Chicken,
  Demon: Demon,
  Dino: Dino,
  Dog: Dog,
  Dragon: Dragon,
  DragonEvolved: DragonEvolved,
  Fish: Fish,
  Frog: Frog,
  Ghost: Ghost,
  GhostSkull: GhostSkull,
  Glub: Glub,
  GlubEvolved: GlubEvolved,
  Goleling: Goleling,
  GolelingEvolved: GolelingEvolved,
  GreenBlob: GreenBlob,
  GreenSpikyBlob: GreenSpikyBlob,
  Hywirl: Hywirl,
  Monkroose: Monkroose,
  Mushnub: Mushnub,
  MushnubEvolved: MushnubEvolved,
  MushroomKing: MushroomKing,
  Ninja: Ninja,
  Orc: Orc,
  OrcSkull: OrcSkull,
  Pigeon: Pigeon,
  PinkBlob: PinkBlob,
  Squidle: Squidle,
  Tribal: Tribal,
  Wizard: Wizard,
  Yeti: Yeti,
} as { [k: string]: string };

type ModelLoaded = (name: string) => void;

const loadModel = (name: string, url: string, loaded: ModelLoaded) => {
  const loader = new GLTFLoader();
  return new Promise((resolve, reject) => {
    loader.load(
      url,
      (gtlf) => {
        loaded(name);
        resolve([name, gtlf]);
      },
      (event: ProgressEvent) => {
        // console.log(name, event);
      },
      (error: ErrorEvent) => {
        reject(error);
      },
    );
  });
};

export const loadAllModels = async (props: LoadingProps) => {
  var promises = Object.entries(allModels).map((pair) => {
    const [name, url] = pair;
    const rv = loadModel(name, url, props.onModelLoaded);
    return rv;
  });
  var models = await Promise.all(promises);
  var rv = new Map();
  for (var i = 0; i < models.length; i++) {
    var [name, model] = models[i] as [string, any];
    rv.set(name, model);
  }
  props.onLoaded(rv);
  return rv;
};

export interface LoadingProps {
  onModelLoaded(name: string): void;
  onLoaded(assets: Map<string, any>): void;
}

export const loadingMain = (props: LoadingProps) => {
  loadAllModels(props);
};
