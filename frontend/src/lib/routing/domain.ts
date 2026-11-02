import type {Payload} from "$routing/address";
import type {WindowKind} from "$types";

/**
 * 一个窗口的域：地址段 ↔ 目的地 ↔ 视图的唯一翻译者。
 *
 * `nav` 只认这个接口。`decode` / `encode` 必须互逆（`tests/routing.test.mjs` 有往返测试）
 */
export interface Domain<Dest> {
  /** 本窗口名（= 地址第一段）。闭集来自 Rust 的 `WindowKind` */
  readonly host: WindowKind;

  /** 地址段 → 目的地。**严格解释**，失败返回 `null`（降级由 `nav` 统一处理） */
  decode(segments: readonly string[]): Dest | null;

  /** 目的地 → 地址段。允许省略尾部默认段，故只保证 `decode(encode(d)) 等价于 d` */
  encode(dest: Dest): readonly string[];

  /** 落地：目的地 → 视图。`payload` 是一次性命令参数，落地即消费，不从地址重建 */
  land(dest: Dest, payload?: Payload): void;
}
