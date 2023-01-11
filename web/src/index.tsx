import './style.css';

import { DOMcreateElement } from './render';
import { WelcomeScreen } from './components';

const main = () => {
  const el = WelcomeScreen();
  document.body.appendChild(el);
}
main();
