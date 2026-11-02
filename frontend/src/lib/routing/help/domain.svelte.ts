import {type Payload} from "$routing/address";
import type {Domain} from "$routing/domain";
import {HelpAddress} from "$routing/help/address";
import {type History, pushAt, resetTo, stepIndex} from "$routing/help/history";
import {nav} from "$routing/nav";
import {helpStore} from "$stores/helpStore.svelte";

/**
 * 窗口交付给导航的**视口端口**：滚动与折叠树是视图关注点，导航只负责说要做什么。
 * 由窗口根组件在 `install()` 时装入，未安装时这些副作用静默跳过。
 */
export type HelpViewport = {
  /** 滚到锚点；`null` 表示回到文档顶部 */
  scrollTo(anchor: string | null): void;
  /** 折叠导航树到当前路径（侧栏点击不折叠，其余跳转折叠） */
  foldTree(): void;
};

/** 一次落地的意图：由 `open()` 置位，落地时消费即复位 */
type Intent = { push: boolean; fold: boolean };

/** 地址**不是**来自本窗口的点击时的意图：不记栈、折叠树 */
const DELIVERED: Intent = {push: false, fold: true};

/**
 * help 域：地址段 ↔ `HelpAddress` ↔ 视图，并持有地址栈。
 *
 * 落点（折叠树、加载文档、滚锚点、入栈）全在这里：栈与「这次跳转要不要入栈」是同一事实的两面，
 * 拆到组件里必漂移。窗口只留视口端口（`install`），门面见 `navigate.ts`。
 *
 * 栈只记用户点击：侧栏、搜索、文档内链接入栈；跨窗口投递与冷启动载荷把栈重置为这一条
 */
class HelpDomain implements Domain<HelpAddress> {
  readonly host = HelpAddress.HOST;
  #history = $state<History<HelpAddress>>({entries: [], index: 0});
  /** 已落到视图的地址。**同步**于加载之前更新，故树高亮不必等文档就位 */
  #shown = $state<HelpAddress | null>(null);
  #intent: Intent = DELIVERED;
  /** 加载序号：在途的加载是否已被更晚的导航取代，靠它判断 */
  #loading = 0;
  /** 在途锚点：加载期间又发生导航则丢弃这一次的滚动 */
  #pending: string | null = null;
  #viewport: HelpViewport | null = null;

  /** 由窗口根组件安装视口端口，返回卸载函数 */
  install(viewport: HelpViewport): () => void {
    this.#viewport = viewport;
    return () => {
      if (this.#viewport === viewport) {
        this.#viewport = null;
      }
    };
  }

  /** 屏幕上这一篇；树高亮与面包屑都从它派生 */
  get current(): HelpAddress | null {
    return this.#shown;
  }

  get canGoBack(): boolean {
    return stepIndex(this.#history, -1) !== null;
  }

  get canGoForward(): boolean {
    return stepIndex(this.#history, 1) !== null;
  }

  /** 在途锚点非空时，窗口的「切文档回顶部」副作用应让路 */
  get pendingAnchor(): string | null {
    return this.#pending;
  }

  /** 用户点击打开一处（侧栏树、搜索结果、文档内链接）：本次落地入栈 */
  open(address: HelpAddress, options?: { fold?: boolean }): void {
    this.#intent = {push: true, fold: options?.fold ?? true};
    nav.go({route: address.toHash()});
  }

  /** 地址段 → 地址；任何时候都能给出地址（非法锚点降级为文档顶部），故不会返回 null */
  decode(segments: readonly string[]): HelpAddress {
    return HelpAddress.fromSegments(segments);
  }

  /** 地址 → 地址段；与 `decode` 互逆（锚点为空时末段省略） */
  encode(address: HelpAddress): readonly string[] {
    return address.toSegments();
  }

  /**
   * 落地：`nav` 的唯一出口。地址同步由 `dispatch` 在落地成功后统一做，这里不碰 hash。
   *
   * `payload` 是一次性命令参数（如「打开并预填搜索词」），落地即消费，暂无消费者。
   * 幂等：重复投递同一地址只重新定位，不重复加载
   */
  land(address: HelpAddress, payload?: Payload): void {
    void this.#apply(address, payload).catch((error) => {
      console.error("[help] 落地失败", error);
    });
  }

  /** 后退一格：地址与锚点一并还原，不记录新历史 */
  async back(): Promise<void> {
    await this.#step(-1);
  }

  /** 前进一格：不记录新历史 */
  async forward(): Promise<void> {
    await this.#step(1);
  }

  async #apply(address: HelpAddress, _payload?: Payload): Promise<void> {
    const {push, fold} = this.#intent;
    this.#intent = DELIVERED;

    if (address.toHash() === this.#shown?.toHash()) {
      this.#viewport?.scrollTo(address.anchor);
      return;
    }

    const loading = await this.#load(address, fold);
    // 加载期间又被更晚的导航取代：这一条连栈都不进，否则会盖在后来的那次之上
    if (loading !== this.#loading) return;
    this.#history = push ? pushAt(this.#history, address) : resetTo(address);
  }

  /** 栈移动一格并落到视图（hash 与树折叠都按「外部跳转」处理） */
  async #step(delta: number): Promise<void> {
    const index = stepIndex(this.#history, delta);
    if (index === null) {
      return;
    }
    this.#history = {...this.#history, index};
    const address = this.#history.entries[index];
    nav.replaceHash(address.toHash());
    await this.#load(address, true);
  }

  /** 加载文档并滚动到锚点；返回本次加载的序号，供调用方判断是否已被更晚的导航取代 */
  async #load(address: HelpAddress, fold: boolean): Promise<number> {
    this.#shown = address;
    if (fold) {
      this.#viewport?.foldTree();
    }
    const loading = (this.#loading += 1);
    this.#pending = address.anchor;
    await helpStore.load(address.doc);
    if (loading !== this.#loading) return loading;
    this.#viewport?.scrollTo(address.anchor);
    this.#pending = null;
    return loading;
  }
}

export const helpDomain = new HelpDomain();
