/**
 * 当前窗口的路由单例
 *
 * 窗口的根组件（`App.svelte` / `Help.svelte`）创建 router 后执行 `install()`，
 * 之后窗口里**任何组件、store、service** 都能用 `WindowRouter.nav(action)` 发起跳转，
 * 不必层层传 props、也不必各自认得路由对象。
 * 未安装时静默忽略。
 */
import {type Action, isRouteHref, type Route} from "$routing/address";
import type {Router} from "$routing/router";

let router: Router | null = null;

export const WindowRouter = {
  /** 由窗口根组件安装，重复安装以最后一次为准 */
  install(next: Router): void {
    router = next;
  },

  /** 发起跳转：判归属与跨窗口转发都在 router 内部完成 */
  nav(action: Action): void {
    router?.go(action);
  },

  replaceHash(route: Route): void {
    /**
     * 只同步 `location.hash`，**不触发导航**——导航的落地由窗口自己的 handler 负责，hash 只是它的对外投影。
     *
     * 使用 `replaceState` 而非赋值 `location.hash`：后者会产生浏览器历史条目，与各窗口自有的前进/后退栈发生冲突；
     * 且 `replaceState` **不触发 hashchange**，天然不会把"自己写的地址"再处理一遍。
     */
    if (!isRouteHref(route)) {
      return
    }
    history.replaceState(null, "", route);
  },
};
