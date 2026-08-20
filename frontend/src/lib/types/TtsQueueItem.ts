import {TtsPhase} from "$types/TtsPhase";
import {TtsMode} from "$types/TtsMode";

/** 播报队列项 */
export type TtsQueueItem = {
  id: string;
  studentNo: string;
  name: string;
  status: TtsPhase;
  /** 入队时固化当前 TtsMode 快照，播放/加载完全依据此字段，忽略全局 mode */
  generatedMode: TtsMode;
  audioUrl?: string;
  error?: string;
};
