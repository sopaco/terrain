import "./app.css";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { mount } from "svelte";
import App from "./App.svelte";
import UsageWindow from "./UsageWindow.svelte";

const target = document.getElementById("app")!;

async function bootstrap() {
  const label = getCurrentWindow().label;
  if (label === "usage") {
    mount(UsageWindow, { target });
    return;
  }
  mount(App, { target });
}

void bootstrap();
