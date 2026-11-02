import type {Payload} from "$routing/address";
import type {WindowKind} from "$types";

/**
 * 一个窗口的「域」：地址段 ↔ 目的地 ↔ 视图，三者的唯一翻译者。
 *
 * 两个窗口因此同构，差别只收在各自的 `land` 里 —— app 是"链 diff + 浮层注册表"，
 * help 是"文档加载 + 滚动 + 栈"。`nav` 只认这个接口，不认任何域的具体概念。
 *
 * `decode` / `encode` 必须互逆：这是"同一个位置只有一个地址"的保证，也是
 * `tests/routing.test.mjs` 里那条往返测试在守的东西。
 */
export interface Domain<Dest> {
  /** 本窗口名（= 地址第一段）。闭集来自 Rust 的 `WindowKind` */
  readonly host: WindowKind;

  /** 地址段 → 目的地。**严格解释**，失败返回 `null`（降级由 `nav` 统一处理） */
  decode(segments: readonly string[]): Dest | null;

  /** 目的地 → 地址段。允许省略尾部默认段，故只保证 `decode(encode(d)) 等价于 d` */
  encode(dest: Dest): readonly string[];

  /**
   * 落地：目的地 → 视图。
   *
   * `payload` 只在这一刻有意义 —— 它是**一次性命令参数**，落地即消费，永远不从地址重建。
   */
  land(dest: Dest, payload?: Payload): void;
}
