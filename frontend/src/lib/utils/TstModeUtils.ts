import {TtsMode} from "$types";

export function toTtsMode(id: number): TtsMode {
  switch (id) {
    case -1:
      return "Off";
    case 0:
      return "SystemNative";
    case 1:
      return "AICloud";
  }
  return "Off"
}

export function toId(ttsMode: TtsMode) {
  switch (ttsMode) {
    case "Off":
      return -1;
    case "SystemNative":
      return 0;
    case "AICloud":
      return 1;
  }
  return -1;
}
