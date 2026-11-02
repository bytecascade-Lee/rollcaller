/**
 * 落点（entry）注册表：**窗口内一张表**。
 *
 * ## 谁注册
 *
 * 只有**叠加层**注册。平面的呈现是"当前地址链的链首"的投影（见 `App.svelte` 的转换语义），
 * 不需要 handler —— 这正是"合表不合槽"：统一的是**如何被寻址**，不是**如何被存储**。
 *
 * ## 载荷
 *
 * 载荷的形状由入口在 `meta.json` 里声明（生成成 `AppEntryPayload`），注册表把它转成
 * `open` 的实参类型：浮层写 `open(id: number)`，调用方写 `enter(editStudent, 7)`，
 * **两端都受同一份声明约束**，中间没有手写解析。
 *
 * 唯一必须自己防的是"地址没有载荷"（深链进来只有地址）—— 故实参类型是 `P | undefined`，
 * 每个浮层开头判断一次，通常就是"不开"。
 */
export type EntryHandler<P = undefined> = {
  open: (payload: P | undefined) => void;
  close?: () => void;
};

/** 内部存储用的宽类型：注册时按 `P[E]` 收窄，取出时已无从得知具体入口 */
type LooseHandler = {
  open: (payload?: unknown) => void;
  close?: () => void;
};

export type EntryRegistry<Id extends string, P = Record<Id, undefined>> = {
  register<E extends Id & keyof P>(id: E, handler: EntryHandler<P[E]>): void;
  unregister(id: Id): void;
  /**
   * 由落地流程调用（`App.svelte`）。载荷在这里必须收成 `unknown`：它来自地址，
   * 跨窗口时是 JSON，落地前无从得知具体入口 —— 声明只用于约束**注册**那一侧。
   */
  open(id: Id, payload?: unknown): void;
  close(id: Id): void;
  has(id: Id): boolean;
};

export function createEntryRegistry<Id extends string, P = Record<Id, undefined>>(): EntryRegistry<Id, P> {
  const handlers = new Map<Id, LooseHandler>();
  return {
    register(id, handler) {
      // **本模块唯一的类型逃逸点**：运行时 `Map` 擦掉了 `P`，取出来时已无从得知是哪个入口，
      // 只能在 `open` 处当作 `unknown` 交回去。不是待修的 bug，别试图"修好它"。
      handlers.set(id, handler as unknown as LooseHandler);
    },
    unregister(id) {
      handlers.delete(id);
    },
    open(id, payload) {
      handlers.get(id)?.open(payload);
    },
    close(id) {
      handlers.get(id)?.close?.();
    },
    has(id) {
      return handlers.has(id);
    },
  };
}
