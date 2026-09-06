import {listen, type UnlistenFn} from "@tauri-apps/api/event";
import * as UpdateCommand from "$commands/update";
import {UPDATE_VIEW_EVENT} from "$commands/update";
import type {UpdateView} from "$types/UpdateView";

/**
 * 更新状态 store（单一权威窗口使用）
 *
 * 后端是状态唯一真源；本 store 只做**展示镜像**，且只有一条数据通路：
 * 命令返回的 [`UpdateView`] 与后端广播的同一类型事件 → 都走 `#apply`。
 * 前端不推断阶段；`error` 变体、进度等一律来自后端视图。
 */
export class UpdateStore {
  #view = $state<UpdateView>({status: "idle"});
  #lastError = $state<string | null>(null);
  #unlisten: UnlistenFn | null = null;

  /** 当前展示视图（组件直接消费，无需自行推导阶段） */
  get view() {
    return this.#view;
  }

  /** 最近一次命令被入口拒绝的错误（防重入/环境类，非业务失败） */
  get lastError() {
    return this.#lastError;
  }

  /** 订阅后端广播（幂等：只注册一次） */
  async subscribe() {
    if (this.#unlisten) return;
    this.#unlisten = await listen<UpdateView>(UPDATE_VIEW_EVENT, (event) => {
      this.#apply(event.payload);
    });
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

  /** 下载已批准产物（进度经后端广播实时进入 store） */
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
    this.#view = next;
  }
}

export const updateStore = new UpdateStore();
