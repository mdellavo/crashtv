import { DOMcreateElement } from "./render";
import { css } from "@emotion/css";

export interface LoadingMessageProps {
  id?: string;
  message?: string;
  status?: string;
}

export const LoadingMessage = (props: LoadingMessageProps) => {
  return (
    <div
      id={props.id}
      className={css`
        display: flex;
        justify-content: center;
        align-items: center;
        flex-direction: column;
      `}
    >
      <h1>{props.message || "Loading..."}</h1>
      <p>{props.status || ""}</p>
    </div>
  );
};

export interface WelcomeScreenProps {
  message?: string;
  onJoin(username: string): void;
}

export const WelcomeScreen = (props: WelcomeScreenProps) => {
  let username = `username-${Math.floor(Math.random() * 1000)}`;

  const onSubmit = (e: any) => {
    e.preventDefault();
    props.onJoin(username);
  };

  return (
    <div
      className={css`
        display: flex;
        justify-content: center;
        align-items: center;
        flex-direction: column;
        padding-top: 200px;
        height: 200px;
      `}
    >
      <div>
        <h1>CrashTV</h1>
        <br />
        <form method="post" onsubmit={onSubmit}>
          <div>
            <h2>{props.message || ""}</h2>

            <div>
              <input
                name="username"
                value={username}
                onchange={(e: any) => (username = e.target.value)}
              />
            </div>

            <div
              className={css`
                display: flex;
                align-items: center;
                justify-content: center;
                margin-top: 20px;
              `}
            >
              <button type="submit">Play</button>
            </div>
          </div>
        </form>
      </div>
    </div>
  );
};
