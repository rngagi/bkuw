import React from "react";
import ReactDOM from "react-dom/client";
import App from "./App";
import "./i18n";
import "./App.css";
import { installTextInputPolicy } from "./lib/inputPolicy";

installTextInputPolicy();

if ("__TAURI__" in window) {
  void import("@wdio/tauri-plugin");
}

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode>
    <App />
  </React.StrictMode>,
);
