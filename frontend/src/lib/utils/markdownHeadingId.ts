/**
 * markdown-it 插件：把标题行尾的 `{#kebab-name}` 提为标题的 `id` 属性。
 *
 *   `## 操作步骤 {#operation-steps}`  →  `<h2 id="operation-steps">操作步骤</h2>`
 *
 * 为什么用行内属性而不是 HTML 注释指令：
 *   - 属性随 heading token 走，不需要"行号 → token"的对齐表；
 *   - 普通 markdown 渲染器（GitHub 等）把它当普通文本忽略，不影响阅读；
 *   - 作者写错时 `{#...}` 会**肉眼可见地留在正文里**，而不是静默失效。
 *
 * 只处理标题：不引入 `{.class}` 等其它属性能力，避免与正文里合法的花括号冲突。
 */
// markdown-it v15 自带类型：`MarkdownIt` 是实例类型（named type），默认导出是可调用的构造值
import type {MarkdownIt} from "markdown-it";

/** 标题内容结尾的锚点声明；非 kebab 形式不识别（留作正文，可见即可发现） */
const HEADING_ID_RE = /\s*\{#([a-z0-9]+(?:-[a-z0-9]+)*)\}\s*$/;

export function headingIdPlugin(md: MarkdownIt): void {
  // 必须早于 inline 规则：此时 inline.content 已就绪、children 尚未生成，
  // 改写 content 才会真正影响标题的渲染结果。
  md.core.ruler.before("inline", "heading-id", (state) => {
    for (let i = 0; i < state.tokens.length; i++) {
      const open = state.tokens[i];
      if (open.type !== "heading_open") continue;
      const inline = state.tokens[i + 1];
      if (!inline || inline.type !== "inline") continue;
      const matched = HEADING_ID_RE.exec(inline.content);
      if (!matched) continue;
      inline.content = inline.content.slice(0, matched.index).trimEnd();
      open.attrSet("id", matched[1]);
    }
  });
}
