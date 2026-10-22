// Sends script errors to the app's log, where they can be seen without
// the web inspector.
const { invoke } = window.__TAURI__.core;

function report(message) {
  invoke("log_error", { message: String(message) }).catch(() => {});
}

window.addEventListener("error", (event) => report(`${event.message} (${event.filename}:${event.lineno})`));
window.addEventListener("unhandledrejection", (event) => report(`unhandled rejection: ${event.reason}`));
