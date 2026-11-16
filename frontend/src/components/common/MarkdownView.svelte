<script lang="ts">
  import MarkdownIt from "markdown-it";
  import hljs from "highlight.js/lib/core";
  import bash from "highlight.js/lib/languages/bash";
  import DOMPurify from "dompurify";
  import "highlight.js/styles/github.css";
  import {error} from "@fltsci/tauri-plugin-tracing";
  import {openUrl} from "@tauri-apps/plugin-opener";
  import {HelpAddress} from "$routing/help/address";
  import {headingIdPlugin} from "$utils/markdownHeadingId";

  hljs.registerLanguage("bash", bash);

  let {
    markdown = "",
    onnavigate,
  }: {
    markdown?: string;
    /** 文档内链接跳转回调：由宿主决定"加载 + 定位"如何落地 */
    onnavigate?: (address: HelpAddress) => void;
  } = $props();

  const md = new MarkdownIt({
    html: true,
    linkify: true,
    typographer: true,
    // 只有显式标注且已注册的语言才高亮。未标注语言的围栏按纯文本输出：
    // 返回空串时 markdown-it 会自己转义正文，不要走 highlightAuto —— 本组件只注册了
    // bash，自动识别必然选中它，会把许可证正文里的 "Software"、in 之类染成字符串/关键字。
    highlight: (str, lang) => {
      if (!lang || !hljs.getLanguage(lang)) return "";
      try {
        return hljs.highlight(str, {language: lang}).value;
      } catch (e) {
        error(e instanceof Error ? e.message : String(e));
        return "";
      }
    },
  }).use(headingIdPlugin);

  const defaultLinkOpen =
    md.renderer.rules.link_open ?? ((tokens, idx, options, _env, self) => self.renderToken(tokens, idx, options));

  // 渲染期只做一件事：把作者写的文档相对链接改写成应用内路由。
  // 外部链接与同文档锚点原样保留，分别交给点击期与浏览器原生处理。
  md.renderer.rules.link_open = (tokens, idx, options, env, self) => {
    const token = tokens[idx];
    const address = HelpAddress.fromDocLink(String(token.attrGet("href") ?? ""));
    if (address) token.attrSet("href", address.toHash());
    return defaultLinkOpen(tokens, idx, options, env, self);
  };

  const safeHtml = $derived(
    DOMPurify.sanitize(md.render(markdown), {
      USE_PROFILES: {html: true},
      ADD_ATTR: ["target"],
    }),
  );

  /**
   * 点击期是唯一的解释器，不依赖渲染期写入的任何标记：
   * - 外部链接 → 系统浏览器
   * - `#/HELP/...` → 交给宿主导航
   * - 其余（同文档锚点等）→ 完全不拦截，交浏览器原生处理
   */
  function handleClick(event: MouseEvent) {
    const anchor = (event.target as Element).closest("a");
    if (!anchor) return;
    const href = anchor.getAttribute("href") ?? "";
    if (/^https?:\/\//i.test(href)) {
      event.preventDefault();
      openUrl(href).catch((e) => error(e instanceof Error ? e.message : String(e)));
      return;
    }
    const address = HelpAddress.parse(href);
    if (!address) return;
    event.preventDefault();
    onnavigate?.(address);
  }
</script>

<!-- svelte-ignore a11y_click_events_have_key_events -->
<!-- svelte-ignore a11y_no_static_element_interactions -->
<div class="markdown-body" onclick={handleClick}>
  <!-- eslint-disable-next-line svelte/no-at-html-tags -->
  {@html safeHtml}
</div>

<style>
  .markdown-body {
    font-family: var(--font-family-sans);
    font-size: var(--font-size-sm);
    line-height: 1.5;
    color: var(--text-color-primary);
    padding-right: var(--space-sm);
    overflow-wrap: break-word;
    user-select: none;
  }

  /* 六个标题共用同一套垂直节奏；h5/h6 此前没有样式，落到浏览器默认（反而比正文小） */
  .markdown-body :global(h1),
  .markdown-body :global(h2),
  .markdown-body :global(h3),
  .markdown-body :global(h4),
  .markdown-body :global(h5),
  .markdown-body :global(h6) {
    margin: 1.75rem 0 0.75rem;
    line-height: 1.35;
    font-weight: var(--font-weight-bold);
  }

  .markdown-body :global(h1) {
    margin-top: 0;
    font-size: 1.6rem;
  }

  .markdown-body :global(h2) {
    font-size: 1.35rem;
  }

  .markdown-body :global(h3) {
    font-size: 1.15rem;
  }

  .markdown-body :global(h4) {
    font-size: 1rem;
  }

  .markdown-body :global(h5) {
    font-size: 0.9rem;
  }

  .markdown-body :global(h6) {
    font-size: 0.85rem;
    color: var(--text-color-secondary);
  }

  .markdown-body :global(p) {
    margin: 0 0 1rem;
  }

  .markdown-body :global(a) {
    color: var(--color-primary);
    text-decoration: none;
  }

  .markdown-body :global(a:hover) {
    text-decoration: underline;
  }

  .markdown-body :global(ul),
  .markdown-body :global(ol) {
    margin: 0 0 1rem 1.8rem;
    padding-left: 0;
  }

  .markdown-body :global(li) {
    margin-bottom: 0.25rem;
  }

  .markdown-body :global(li > ul),
  .markdown-body :global(li > ol) {
    margin-bottom: 0;
  }

  .markdown-body :global(blockquote) {
    margin: 0 0 1rem;
    padding: 0 1rem;
    border-left: 4px solid var(--border-color-3);
    color: var(--text-color-secondary);
  }

  /*
   * 代码块强制软换行：许可证正文里有 755 字符的单行段落，不换行会横向溢出。
   * `word-break: normal` 与 `overflow-wrap: break-word` 配合 —— 只断开确实放不下的长
   * 单词（URL、哈希），不按任意字符切分。`tab-size` 让缩进保持源码原意。
   */
  .markdown-body :global(pre) {
    margin: 0 0 1rem;
    padding: var(--space-xs);
    border-radius: var(--radius-md);
    background: var(--color-card);
    white-space: pre-wrap;
    overflow-wrap: break-word;
    word-break: normal;
    tab-size: 4;
    user-select: text;
  }

  .markdown-body :global(pre code) {
    font-family: var(--font-family-mono);
    font-size: 0.8rem;
    line-height: 1.6;
    padding: 0;
    white-space: inherit;
    overflow-wrap: inherit;
    word-break: inherit;
    user-select: inherit;
  }

  /* 内联码用 em 而非 rem：跟着所在文本缩放，放进 h2 里不会突然缩水 */
  .markdown-body :global(p code),
  .markdown-body :global(li code),
  .markdown-body :global(th code),
  .markdown-body :global(td code) {
    padding: 0.1rem 0.3rem;
    border-radius: var(--radius-md);
    background: var(--color-card);
    font-family: var(--font-family-mono);
    font-size: 0.9em;
    user-select: text;
  }

  /*
   * 表格：
   * - `width: auto` + `max-width: 100%` —— 宽表照样吃满，短表按内容宽收拢。
   *   若写成 `width: 100%`，两列表会被拉满，「组件数」这种数字列会空出半屏，
   *   而浏览器把多余宽度按比例摊给各列，短列（版本号）也就跟着虚胖。
   * - `th` 不折行 —— CJK 默认按字断开，窄列会把「序号」竖排成两行
   * - `break-word` 只影响放不下的长单词，不参与 min-content 计算，因此不会把短列压成一字宽
   * - `vertical-align: top` 让多行的版权列与短值单元格顶部对齐
   */
  .markdown-body :global(table) {
    border-collapse: collapse;
    margin: 0 0 1rem;
    width: auto;
    max-width: 100%;
  }

  .markdown-body :global(th),
  .markdown-body :global(td) {
    padding: 0.45rem 0.7rem;
    border: 1px solid var(--border-color-4);
    text-align: left;
    vertical-align: top;
  }

  .markdown-body :global(th) {
    font-weight: var(--font-weight-bold);
    white-space: nowrap;
  }

  /* 斑马纹：底色与页面同色（#f5f5f5）时整条规则是隐形的，故取 gray-2 */
  .markdown-body :global(tr:nth-child(even)) {
    background: var(--color-background);
  }

  .markdown-body :global(hr) {
    border: 0;
    border-top: 2px solid var(--border-color-2);
    margin: 1.5rem 0;
  }

  .markdown-body :global(img) {
    max-width: 100%;
    height: auto;
    border-radius: var(--radius-sm);
  }

  .markdown-body :global(strong) {
    font-weight: var(--font-weight-bold);
  }

  .markdown-body :global(em) {
    font-style: italic;
  }
</style>
