import React from "react";
import ReactDOM from "react-dom/client";
import App from "./App";
import { currentLocale } from "./i18n/i18n";

// F34: leitores de tela e hifenização seguem o idioma do app.
document.documentElement.lang = currentLocale();

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode>
    <App />
  </React.StrictMode>,
);
