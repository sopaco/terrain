import "./app.css";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { mount } from "svelte";
import App from "./App.svelte";
import UsageWindow from "./UsageWindow.svelte";
import { loadAppBootstrap } from "./lib/appBootstrap";
import { applyLocale } from "./lib/i18n";

const target = document.getElementById("app")!;

async function bootstrap() {
  try {
    const boot = await loadAppBootstrap();
    applyLocale(boot.model_settings.language);
  } catch {
    applyLocale("system");
  }

  const label = getCurrentWindow().label;
  if (label === "usage") {
    mount(UsageWindow, { target });
    return;
  }
  mount(App, { target });
}

void bootstrap();
