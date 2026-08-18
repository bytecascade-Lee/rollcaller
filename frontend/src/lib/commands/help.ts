import {invoke} from "@tauri-apps/api/core";

export async function markdown(id: string) {
  return await invoke<string>("help_load_markdown", {
    id: id,
  })
}

export async function readme() {
  return await invoke<string>("help_load_readme", {})
}

export async function license() {
  return await invoke<string>("help_load_license", {})
}

export async function changelog() {
  return await invoke<string>("help_load_changelog", {})
}

export async function releaseNotes() {
  return await invoke<string>("help_load_release_notes", {})
}
