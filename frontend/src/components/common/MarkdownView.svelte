<script lang="ts">
  import MarkdownIt from "markdown-it";
  import hljs from "highlight.js/lib/core";
  import bash from "highlight.js/lib/languages/bash";
  import DOMPurify from "dompurify";
  import "highlight.js/styles/github.css";
  import {error} from "@fltsci/tauri-plugin-tracing";
  import {openUrl} from "@tauri-apps/plugin-opener";
  import {HelpAddress} from "$utils/helpAddress";
  import {headingIdPlugin} from "$utils/markdownHeadingId";

  hljs.registerLanguage("bash", bash);

  let {markdown = "", onnavigate}: {
    markdown?: string;
    /** 文档内链接跳转回调：由宿主决定"加载 + 定位"如何落地 */
    onnavigate?: (address: HelpAddress) => void;
  } = $props();

  const md = new MarkdownIt({
    html: true,
    linkify: true,
    typographer: true,
    highlight: (str, lang) => {
      if (lang && hljs.getLanguage(lang)) {
        try {
          return hljs.highlight(str, {language: lang}).value;
        } catch (e) {
          error(e instanceof Error ? e.message : String(e));
          return "";
        }
      }
      try {
        return hljs.highlightAuto(str).value;
      } catch (e) {
        error(e instanceof Error ? e.message : String(e));
        return "";
      }
    },
  }).use(headingIdPlugin);

  const defaultLinkOpen = md.renderer.rules.link_open ??
    ((tokens, idx, options, _env, self) => self.renderToken(tokens, idx, options));

  // 渲染期只做一件事：把作者写的文档相对链接改写成应用内路由。
  // 外部链接与同文档锚点原样保留，分别交给点击期与浏览器原生处理。
  md.renderer.rules.link_open = (tokens, idx, options, env, self) => {
    const token = tokens[idx];
    const address = HelpAddress.fromDocLink(String(token.attrGet("href") ?? ""));
    if (address) token.attrSet("href", address.toHash());
    return defaultLinkOpen(tokens, idx, options, env, self);
  };

  const safeHtml = $derived(DOMPurify.sanitize(md.render(markdown), {
    USE_PROFILES: {html: true},
    ADD_ATTR: ["target"],
  }));

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
<div class="markdown-body" onclick={handleClick} style="background: transparent;">
  {@html safeHtml}
</div>

<style>
  .markdown-body {
    font-family: var(--font-family-sans);
    font-size: var(--font-size-sm);
    line-height: 1.5;
    color: var(--text-color-primary);
    padding-right: var(--space-sm);
    background: var(--color-page);
    overflow-wrap: break-word;
    user-select: none;
  }

  .markdown-body :global(h1) {
    font-size: 1.6rem;
    padding-top: 0;
    font-weight: var(--font-weight-bold);
  }

  .markdown-body :global(h2) {
    font-size: 1.35rem;
    font-weight: var(--font-weight-bold);
  }

  .markdown-body :global(h3) {
    font-size: 1.15rem;
    font-weight: var(--font-weight-bold);
  }

  .markdown-body :global(h4) {
    font-size: 1rem;
    font-weight: var(--font-weight-bold);
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

  .markdown-body :global(pre) {
    padding: var(--space-xs);
    border-radius: var(--radius-md);
    background: var(--color-card);
    white-space: pre-wrap;
    word-break: break-word;
    overflow-wrap: break-word;
  }

  .markdown-body :global(pre code) {
    font-family: var(--font-family-mono);
    font-size: 0.8rem;
    line-height: 1.6;
    padding: 0;
    white-space: pre-wrap;
    word-break: break-word;
  }

  .markdown-body :global(p code),
  .markdown-body :global(li code) {
    padding: 0.15rem 0.4rem;
    border-radius: var(--radius-md);
    background: var(--color-card);
    font-family: var(--font-family-mono);
    font-size: 0.8rem;
  }

  .markdown-body :global(table) {
    border-collapse: collapse;
    margin: 0 0 1rem;
    border-radius: var(--radius-md);
    width: 100%;
  }

  .markdown-body :global(th),
  .markdown-body :global(td) {
    padding: 0.5rem 1rem;
    border: 1px solid var(--border-color-4);
    text-align: left;
  }

  .markdown-body :global(th li code),
  .markdown-body :global(td li code) {
    font-family: var(--font-family-mono);
  }

  .markdown-body :global(th) {
    font-weight: var(--font-weight-bold);
  }

  .markdown-body :global(tr:nth-child(even)) {
    background: var(--color-page);
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
