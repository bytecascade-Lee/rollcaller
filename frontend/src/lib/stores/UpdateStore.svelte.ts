import {listen, type UnlistenFn} from "@tauri-apps/api/event";
import * as UpdateCommand from "$commands/update";
import {DOWNLOAD_PROGRESS_EVENT, UPDATE_VIEW_EVENT} from "$commands/update";
import type {DownloadProgress} from "$types/DownloadProgress";
import type {UpdateView} from "$types/UpdateView";

/**
 * 更新状态 store（镜像后端唯一真源）
 *
 * 后端状态机是唯一真源；本 store 只做**展示镜像**。两条事件通道对称两条数据槽：
 *
 * - view 通道（[`UPDATE_VIEW_EVENT`]）：状态迁移帧（含进入 Downloading 这一次）与
 *   各命令终态 → `#view`（tagged union，组件据此渲染阶段，不自行推断）；
 * - download 通道（[`DOWNLOAD_PROGRESS_EVENT`]）：Downloading 期的进度窄帧 → `#progress`，
 *   **仅当当前处于 `downloading` 时接受**——终态广播后迟到的陈旧帧一律丢弃。
 *   通道拆分的目的就是把"状态"与"瞬时进度"隔离：进度帧永远不会把 UI 打回下载中。
 *
 * 进度是瞬时数据，只活在 Downloading 期：`#apply` 在进入 / 离开 downloading 时
 * 重置 `#progress`（避免显示上一轮残留 / 把残留带给下一轮）。
 *
 * TODO(设置窗口)：当前仅主窗口（标题栏）渲染并初始化本 store；后期设置窗口加入
 * "检查更新"入口时，各窗口将各自持有镜像实例并订阅广播（后端广播本就喂全窗口），
 * 届时重审 autoCheck 归属与多实例幂等订阅。
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
    this.#run(() => UpdateCommand.check());
  }

  /** 下载已批准产物（进度经 download 通道窄帧进入 store） */
  async download() {
    this.#run(() => UpdateCommand.download());
  }

  /** 取消下载 / 放弃已下载产物 */
  async cancel() {
    this.#run(() => UpdateCommand.cancel());
  }

  /** 安装已下载产物（成功路径进程退出） */
  async install() {
    this.#run(() => UpdateCommand.install());
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
      const leavingOrEntering =
        this.#view.status === "downloading" || next.status === "downloading";
      if (leavingOrEntering) this.#resetProgress();
    }
    this.#view = next;
  }

  #resetProgress() {
    this.#progress = {downloaded: 0, total: null};
  }
}

export const updateStore = new UpdateStore();
