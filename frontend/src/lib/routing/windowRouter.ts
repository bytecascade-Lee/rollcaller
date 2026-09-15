/**
 * 本窗口的路由单例。
 *
 * 一个窗口 = 一个 JS 上下文，所以"模块级单例"天然就是**窗口级**的——不会跨窗口串味。
 * 窗口的根组件（`App.svelte` / `Help.svelte`）创建 router 后 `installWindowRouter()` 一次，
 * 之后窗口里**任何组件**都能用 `nav(action)` 发起跳转，不必层层传 props、也不必各自
 * 认得路由对象。
 *
 * 这就是"发起方只学一个函数"的落点：按钮、搜索结果、跨窗口投递的落地，写的是同一行。
 *
 * 未安装时静默忽略（不报错、不半应用）——与"非法地址忽略""锚点缺失回顶部"同一种降级。
 */
import type {Action} from "$routing/route";
import type {Router} from "$routing/router";

let router: Router | null = null;

/** 由窗口根组件安装（重复安装以最后一次为准，便于热重载） */
export function installWindowRouter(next: Router): void {
  router = next;
}

/** 应用代码的唯一入口 */
export function nav(action: Action): void {
  router?.go(action);
}
