import "./style.css";

import { DOMcreateElement } from "./render";
import { WelcomeScreen, LoadingMessage } from "./components";
import { gameMain } from "./game";
import { loadingMain } from "./models";

const setBody = (el: any) => {
  document.body.innerHTML = "";
  document.body.appendChild(el);
};

const main = () => {
  const loadingStart = Date.now();
  const onLoaded = (assets: Map<string, any>) => {
    const loadingStop = Date.now();
    const delta = loadingStop - loadingStart;

    console.log("loaded in", delta / 1000);

    const goToWelcome = (message?: string) => {
      const el = <WelcomeScreen onJoin={onJoin} message={message} />;

      setBody(el);
    };

    const gameProps = {
      onNotice: (message: string) => {},
      onClose: () => {
        goToWelcome("Disconnected from the server :(");
      },
      onError: (error: any) => {
        goToWelcome(`Error: {error}`);
      },
    };

    const onJoin = (username: string) => {
      console.log("Join", username);
      document.body.innerHTML = "";

      const loading = <LoadingMessage message="Joining..." id="loading" />;
      setBody(loading);

      gameMain(username, gameProps, assets);
    };

    goToWelcome();
  };

  const loading = <LoadingMessage />;
  setBody(loading);

  const onModelLoaded = (name: string) => {
    const msg = `loaded ${name}`;
    const loading = <LoadingMessage status={msg} />;
    setBody(loading);
  };

  loadingMain({
    onModelLoaded: onModelLoaded,
    onLoaded: onLoaded,
  });
};
main();
