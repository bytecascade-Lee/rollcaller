import {listen, type UnlistenFn} from "@tauri-apps/api/event";
import * as UpdateCommand from "$commands/updater";
import {DOWNLOAD_PROGRESS_EVENT, UPDATE_VIEW_EVENT} from "$commands/updater";
import type {DownloadProgress} from "$types/DownloadProgress";
import type {UpdateView} from "$types/UpdateView";

/**
 * 更新状态 store
 *
 * [TODO]：当前仅主窗口（`App.svelte`）初始化本 store；后期设置窗口加入
 *  "检查更新"入口时，各窗口将各自持有镜像实例并订阅广播（后端广播本就喂全窗口），
 *  届时重审自动检查的归属（今天在 `UpdateUiStore.bootstrap`）与多实例幂等订阅。
 */
export class UpdateStore {
  #view = $state<UpdateView>({status: "idle"});
  #progress = $state<DownloadProgress>({downloaded: 0, total: null});
  #lastError = $state<string | null>(null);
  #unlisten: UnlistenFn | null = null;

  /** 当前展示视图（组件直接消费，无需自行推导阶段） */
  get view() {
    return this.#view;
  }

  /** 下载进度（仅 downloading 期有效；进出 downloading 时重置） */
  get progress() {
    return this.#progress;
  }

  /** 最近一次命令被入口拒绝的错误（防重入/环境类，非业务失败） */
  get lastError() {
    return this.#lastError;
  }

  /** 订阅后端广播（幂等：只注册一次；view + download 双通道一并释放） */
  async subscribe() {
    if (this.#unlisten) return;
    const unlistenView = await listen<UpdateView>(UPDATE_VIEW_EVENT, (event) => {
      this.#apply(event.payload);
    });
    const unlistenProgress = await listen<DownloadProgress>(DOWNLOAD_PROGRESS_EVENT, (event) => {
      // 陈旧帧防护：仅下载中接受进度帧；终态后迟到的帧一律作废
      if (this.#view.status !== "downloading") return;
      this.#progress = event.payload;
    });
    this.#unlisten = () => {
      unlistenView();
      unlistenProgress();
    };
  }

  /** 挂载初始化：订阅广播 + 拉一次当前视图（无网络、无副作用） */
  async init() {
    await this.subscribe();
    await this.refresh();
  }

  /** 拉取当前视图（页面挂载/恢复现场用） */
  async refresh() {
    this.#apply(await UpdateCommand.state());
  }

  /** 检查是否有可用更新（自动检查与按钮共用此入口） */
  async check() {
    await this.#run(() => UpdateCommand.check());
  }

  /** 下载已批准产物（进度经 download 通道窄帧进入 store） */
  async download() {
    await this.#run(() => UpdateCommand.download());
  }

  /** 取消下载 / 放弃已下载产物 */
  async cancel() {
    await this.#run(() => UpdateCommand.cancel());
  }

  /** 安装已下载产物（成功路径进程退出） */
  async install() {
    await this.#run(() => UpdateCommand.install());
  }

  /** 统一执行：命令返回/广播同一类型；`Err`（入口拒绝）仅记录提示 */
  async #run(action: () => Promise<UpdateView>) {
    this.#lastError = null;
    try {
      this.#apply(await action());
    } catch (e) {
      this.#lastError = String(e);
    }
  }

  #apply(next: UpdateView) {
    // 进度瞬时语义：进出 downloading 均清零，避免残留/陈旧值串到下一轮
    if (this.#view.status !== next.status) {
      const leavingOrEntering = this.#view.status === "downloading" || next.status === "downloading";
      if (leavingOrEntering) this.#resetProgress();
    }
    this.#view = next;
  }

  #resetProgress() {
    this.#progress = {downloaded: 0, total: null};
  }
}

/** 更新弹窗的开关状态 */
export class UpdateUiStore {
  #open = $state(false);

  /** 弹窗是否已打开 */
  get open() {
    return this.#open;
  }

  /** 打开 / 关闭 / 翻转 */
  show() {
    this.#open = true;
  }

  hide() {
    this.#open = false;
  }

  toggle() {
    this.#open = !this.#open;
  }

  /** 挂载初始化：订阅广播 -> 恢复现场 -> 只在空闲时自动检查一次 */
  async bootstrap() {
    await updateStore.init();
    if (updateStore.view.status === "idle") await updateStore.check();
  }
}

export const updateStore = new UpdateStore();
export const updateUiStore = new UpdateUiStore();
