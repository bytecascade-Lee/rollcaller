/**
 * 落点注册表：每窗口一张。
 *
 * 只有叠加层注册；平面呈现为当前地址链的链首投影，不需要 handler。
 *
 * 载荷形状由 `meta.json` 声明，`register` 据此收窄 `open` 的实参 ⇒ 实参为 `P | undefined`：
 * 地址可以没有载荷（深链只有地址），每个浮层自行判断
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
  /** 由 `applyTransition` 调用。载荷来自地址（跨窗口时为 JSON），故只能是 `unknown` */
  open(id: Id, payload?: unknown): void;
  close(id: Id): void;
  has(id: Id): boolean;
};

export function createEntryRegistry<Id extends string, P = Record<Id, undefined>>(): EntryRegistry<Id, P> {
  const handlers = new Map<Id, LooseHandler>();
  return {
    register(id, handler) {
      // **本模块唯一的类型逃逸点**：运行时 `Map` 擦掉了 `P`，取出来时已无从得知是哪个入口，
      // 只能在 `open` 处当作 `unknown` 交回去。不是待修的 bug，别试图「修好它」。
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
