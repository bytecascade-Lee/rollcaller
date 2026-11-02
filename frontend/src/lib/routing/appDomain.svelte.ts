import type {Payload} from "$routing/address";
import {APP_HOST, type AppEntryId} from "$routing/app.entries.generated";
import type {Domain} from "$routing/domain";
import {resolveAppChain} from "$routing/entries";
import {applyTransition, type LandingSink} from "$routing/transition";

/**
 * app 域：地址段 ↔ 入口链 ↔ 浮层。
 *
 * `#shown` 就是"上一次落地的链"，取代了从前挂在 `App.svelte` 里的 `activeChain`；
 * 卡片激活态与浮层开关都从 `chain` 派生，于是"落地"只有一处状态写入点。
 *
 * 必须是 `.svelte.ts`：`#shown` 是 `$state`，组件要能订阅它。
 */
class AppDomain implements Domain<AppEntryId[]> {
  readonly host = APP_HOST;

  /** 落点注册表由窗口根组件创建（它要拿到组件树），只能装进来 */
  #sink: LandingSink | null = null;
  /** 已落地的链（根 → 当前浮层） */
  #shown = $state<AppEntryId[]>([]);

  /** 装入注册表，返回卸载函数 */
  install(sink: LandingSink): () => void {
    this.#sink = sink;
    return () => {
      if (this.#sink === sink) {
        this.#sink = null;
      }
    };
  }

  /** 当前链；`[0]` 就是当前卡片 */
  get chain(): AppEntryId[] {
    return this.#shown;
  }

  decode(segments: readonly string[]): AppEntryId[] | null {
    return resolveAppChain(segments);
  }

  /** 链本身就是地址段，无需转换（保留此方法是为了与 `decode` 成对，互逆性可测） */
  encode(chain: AppEntryId[]): readonly string[] {
    return chain;
  }

  land(chain: AppEntryId[], payload?: Payload): void {
    if (this.#sink === null) {
      console.warn("[routing] app 域尚未装入注册表，无法落地");
      return;
    }
    this.#shown = applyTransition(this.#sink, this.#shown, chain, payload);
  }
}

export const appDomain = new AppDomain();
