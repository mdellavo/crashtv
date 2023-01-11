import { DOMcreateElement } from './render';
import { css } from '@emotion/css'

export const WelcomeScreen = () => {
  return (
    <div className={css`
      display: flex;
      justify-content: center;
      align-items: center;
      `}>
      <h1>CrashTV</h1>

      <div>
        <input name="username" />
        <br />
        <button type="submit">Play</button>
      </div>
    </div>
  );
}
