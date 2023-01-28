import './style.css';

import { DOMcreateElement } from './render';
import { WelcomeScreen } from './components';
import { gameMain, GameProps } from './game';

const main = () => {

  const goToWelcome = (message?: string) => {
    const el = (
      <WelcomeScreen onJoin={onJoin} message={message}/>
    );
    document.body.innerHTML = "";
    document.body.appendChild(el);
  };

  const props = {
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
    gameMain(username, props);
  };

  goToWelcome();
}
main();
