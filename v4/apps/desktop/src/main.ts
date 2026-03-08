import { mount } from "svelte";
import App from "./App.svelte";

console.debug("[main.ts] App initialization started");

window.addEventListener("error", (e) => {
  console.error("[GLOBAL ERROR]", e.error || e.message);
});

window.addEventListener("unhandledrejection", (e) => {
  console.error("[GLOBAL UNHANDLED REJECTION]", e.reason);
});

const app = mount(App, { target: document.getElementById("app")! });
console.debug("[main.ts] App mounted successfully");

export default app;
