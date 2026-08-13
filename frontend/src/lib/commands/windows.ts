import {invoke} from "@tauri-apps/api/core";

export async function openMainWindow() {
  await invoke<void>("windows_main_open", {});
}

export async function openHelpWindow() {
  await invoke<void>("windows_help_open", {});
}

export async function hideHelpWindow() {
  await invoke<void>("windows_help_hide", {});
}

export async function closeHelpWindow() {
  await invoke<void>("windows_help_close", {});
}

export async function destroyHelpWindow() {
  await invoke<void>("windows_help_destroy", {});
}
