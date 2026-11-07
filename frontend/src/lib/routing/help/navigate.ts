import type {HelpAddress} from "$routing/help/address";
import {helpDomain} from "$routing/help/domain.svelte";

/**
 * help 域的门面：只放**动作**，状态在 `helpDomain`。
 *
 * 与 `appNavigate` 同形（无状态、`Object.freeze`），故窗口侧的用法一致：
 * 动作走门面，读状态走域
 */
export const helpNavigate = Object.freeze({
  /** 用户点击打开一处（侧栏树、搜索结果、文档内链接）：本次落地入栈 */
  open(address: HelpAddress, options?: {fold?: boolean}): void {
    helpDomain.open(address, options);
  },

  /** 后退一格：地址与锚点一并还原 */
  back(): Promise<void> {
    return helpDomain.back();
  },

  /** 前进一格 */
  forward(): Promise<void> {
    return helpDomain.forward();
  },

  /** 打开搜索框并预填：搜索是瞬态过程，没有地址，故不进 `Action` */
  openSearch(query = ""): void {
    helpDomain.openSearch(query);
  },
});
