import * as UpdateCommand from "$commands/update";
import type {DownloadProgressFn} from "$commands/update";
import type {UpdateState} from "$types/UpdateState";

/** 初始空闲快照（后端 state 查询前的占位） */
function idleSnapshot(): UpdateState {
  return {
    status: "idle",
    info: null,
    severity: "normal",
    force: false,
    downloaded: undefined,
    total: undefined,
    error: null,
    errorKind: null,
  };
}

/**
 * 更新状态 store（唯一权威窗口使用，其它窗口不展示更新不订阅）
 *
 * 后端 `state::update::UpdaterState` 是权威执行状态；本 store 只做镜像：
 * 每个命令调用后以后端返回的快照覆盖。`download` / `check` 采用
 * "乐观置位 + 返回校准"：调用前先置对应阶段，进度经 Channel 实时写入，
 * 命令返回后以后端最终快照为准。
 */
export class UpdateStore {
  #snapshot = $state<UpdateState>(idleSnapshot());

  get snapshot() {
    return this.#snapshot;
  }

  /** 页面挂载时拉取一次当前快照（无网络、无副作用） */
  async refresh() {
    this.#apply(await UpdateCommand.state());
  }

  /** 检查更新（乐观置 Checking，返回后校准） */
  async check() {
    this.#snapshot = {...this.#snapshot, status: "checking", error: null, errorKind: null};
    this.#apply(await UpdateCommand.check());
  }

  /** 下载已批准产物（乐观置 Downloading，进度实时写入，结束校准） */
  async download() {
    this.#snapshot = {...this.#snapshot, status: "downloading", downloaded: 0, total: undefined};
    const onProgress: DownloadProgressFn = (downloaded, total) => {
      this.#snapshot = {...this.#snapshot, status: "downloading", downloaded, total: total ?? undefined};
    };
    this.#apply(await UpdateCommand.download(onProgress));
  }

  /** 取消下载 / 放弃已下载产物 */
  async cancel() {
    this.#apply(await UpdateCommand.cancel());
  }

  /** 安装已下载产物（成功路径进程退出） */
  async install() {
    this.#apply(await UpdateCommand.install());
  }

  #apply(next: UpdateState) {
    this.#snapshot = next;
  }
}

export const updateStore = new UpdateStore();
