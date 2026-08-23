/**
 * 帮助文档内链分类：渲染期把 markdown 链接归类为文档链接 / 外部链接 / 其他。
 * 与 MarkdownView 渲染器规则共用，保证测试与运行一致。
 */

export type LinkAction = {kind: "docs"; id: string} | {kind: "external"} | null;

/**
 * - https?:// → 外部链接（保留 href，交给系统浏览器打开）
 * - #fragment（仅同文档）→ 文档链接，指向当前文档内的锚点（如发布说明的目录）
 * - README / CHANGELOG / RELEASE_NOTES / LICENSE → 特殊文档链接
 * - ../<id>/<id>-zh-CN.md[#fragment] → 文档链接；目录名与文件名前缀必须一致，
 *   语言后缀（-zh-CN、-en-US 等）可选
 * - fragment 基于 slug（依赖语言），应用内弃用；滚动目标由 @link 注入的 data-section 决定
 * - 其余（协议内路径等）→ 不分类，保留默认行为
 */
export function classifyLink(href: string, docId?: string): LinkAction {
  if (/^https?:\/\//i.test(href)) return {kind: "external"};
  if (docId && href.startsWith("#")) return {kind: "docs", id: docId};
  const special = href.match(/(README(?:-en-US)?|CHANGELOG|RELEASE_NOTES|LICENSE)(?:\.md)?$/i);
  if (special) return {kind: "docs", id: special[0]};
  const doc = href.match(/\.\.\/([^/]+)\/\1(?:-[a-z]{2}-[A-Z]{2})?\.md(?:#[\s\S]*)?$/i);
  if (doc) return {kind: "docs", id: doc[1]};
  return null;
}
