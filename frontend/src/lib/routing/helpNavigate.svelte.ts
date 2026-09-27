import {type Action} from "$routing/address";
import {WindowRouter} from "$routing/windowRouter";
import {helpStore} from "$stores/helpStore.svelte";
import {HelpAddress} from "$utils/helpAddress";

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

/** 一次落地的意图：由 `open()` 置位，`apply()` 消费即复位 */
type Intent = { record: boolean; fold: boolean };

/** 地址**不是**来自本窗口的点击时的意图：不记栈、折叠树 */
const DELIVERED: Intent = {record: false, fold: true};

/**
 * help 域的导航
 *
 * ## 为什么接管落点
 *
 * 地址栈与"这次跳转要不要入栈"是同一个事实的两面，分散在组件里就会各自漂移：
 * 组件负责加载与滚动、类负责栈，则后退一格该不该重新加载、重复投递该不该入栈，
 * 都没有唯一答案。所以这里一并拿走：写 hash、折叠树、加载文档、滚动锚点、入栈/裁剪。
 * 窗口只剩两件视图事 —— 交出滚动容器（`install`），以及让开"切文档回顶部"的副作用。
 *
 * ## 栈只记用户点击
 *
 * 侧栏树、搜索结果、文档内链接是**点击**，入栈；跨窗口投递与冷启动载荷是**送达**，
 * 不入栈 —— 它们把栈重置成只剩这一条，于是"上一步"不会退到一个本窗口没主动去过的位置。
 *
 * 单例：窗口只有一份地址，与 `WindowRouter` 一样是模块级单例。
 */
class HelpNavigate {
  #stack = $state<HelpAddress[]>([]);
  /** 栈顶下标；新点击裁掉它之后的部分 */
  #index = $state(0);
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
    return this.#index > 0;
  }

  get canGoForward(): boolean {
    return this.#index < this.#stack.length - 1;
  }

  /** 在途锚点非空时，窗口的"切文档回顶部"副作用应让路（见 `Help.svelte`） */
  get pendingAnchor(): string | null {
    return this.#pending;
  }

  /** **用户点击**打开一处（侧栏树、搜索结果、文档内链接）—— 这一次落地入栈 */
  open(address: HelpAddress, options?: { fold?: boolean }): void {
    this.#intent = {record: true, fold: options?.fold ?? true};
    const action: Action = {route: address.toHash()};
    WindowRouter.nav(action);
  }

  /**
   * 地址落地 —— router 的唯一出口。
   *
   * 幂等：重复投递同一地址只重新定位，不重复加载（连点也不会把栈灌满）。
   */
  async apply(address: HelpAddress): Promise<void> {
    const {record, fold} = this.#intent;
    this.#intent = DELIVERED;

    WindowRouter.replaceHash(address.toHash());
    if (address.toHash() === this.#shown?.toHash()) {
      this.#viewport?.scrollTo(address.anchor);
      return;
    }

    const loading = await this.#load(address, fold);
    // 加载期间又被更晚的导航取代：这一条连栈都不进，否则会盖在后来的那次之上
    if (loading !== this.#loading) return;
    if (record) {
      this.#push(address);
    } else {
      this.#stack = [address];
      this.#index = 0;
    }
  }

  /** 后退一格：地址与锚点一并还原，不记录新历史 */
  async back(): Promise<void> {
    if (!this.canGoBack) return;
    this.#index -= 1;
    await this.#show(this.#stack[this.#index]);
  }

  /** 前进一格：不记录新历史 */
  async forward(): Promise<void> {
    if (!this.canGoForward) return;
    this.#index += 1;
    await this.#show(this.#stack[this.#index]);
  }

  /** 把栈顶那一格重新落到视图（hash 与树折叠都按"外部跳转"处理） */
  async #show(address: HelpAddress): Promise<void> {
    WindowRouter.replaceHash(address.toHash());
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

  /** 追加一条并裁掉前进段 */
  #push(address: HelpAddress): void {
    this.#stack = [...this.#stack.slice(0, this.#index + 1), address];
    this.#index = this.#stack.length - 1;
  }
}

export const helpNavigate = new HelpNavigate();
