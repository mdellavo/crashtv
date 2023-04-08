import './style.css';

import { DOMcreateElement } from './render';
import { WelcomeScreen } from './components';
import { gameMain } from './game';
import { loadingMain } from './models';

const main = () => {

  const loadingStart = Date.now();
  const onLoaded = (assets: Map<string, any>) => {
    const loadingStop = Date.now();
    const delta = loadingStop - loadingStart;

    console.log("loaded in", delta / 1000);

    const goToWelcome = (message?: string) => {
      const el = (
        <WelcomeScreen onJoin={onJoin} message={message} />
      );
      document.body.innerHTML = "";
      document.body.appendChild(el);
    };

    const gameProps = {
      onNotice: (message: string) => {

      },
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
      gameMain(username, gameProps, assets);
    };

    goToWelcome();
  };

  loadingMain({
    onLoaded: onLoaded,
  });
}
main();
