function tauriCore() {
  if (typeof window === "undefined") {
    return null;
  }

  if (window.__TAURI__ && window.__TAURI__.core && typeof window.__TAURI__.core.invoke === "function") {
    return window.__TAURI__.core;
  }

  return null;
}

export async function invokeJson(command, argsJson) {
  const core = tauriCore();
  if (!core) {
    throw new Error("Tauri runtime unavailable");
  }

  const args = argsJson ? JSON.parse(argsJson) : {};
  const result = await core.invoke(command, args);
  return JSON.stringify(result);
}

export async function readClipboardText() {
  if (!navigator.clipboard || typeof navigator.clipboard.readText !== "function") {
    throw new Error("Clipboard read is unavailable on this platform");
  }

  return navigator.clipboard.readText();
}

export async function writeClipboardText(text) {
  if (!navigator.clipboard || typeof navigator.clipboard.writeText !== "function") {
    throw new Error("Clipboard write is unavailable on this platform");
  }

  await navigator.clipboard.writeText(text);
}
