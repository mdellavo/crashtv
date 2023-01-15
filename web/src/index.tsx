import './style.css';

import { DOMcreateElement } from './render';
import { WelcomeScreen } from './components';
import { gameMain } from './game';

const main = () => {

  const onJoin = (username: string) => {
    console.log("Join", username);
    document.body.innerHTML = "";
    gameMain(username);
  };

  const el = (
    <WelcomeScreen onJoin={onJoin}/>
  );
  document.body.appendChild(el);
}
main();
