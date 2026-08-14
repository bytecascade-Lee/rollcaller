<script>
  import MarkdownIt from 'markdown-it';
  import hljs from 'highlight.js';
  import DOMPurify from 'dompurify';

  import 'highlight.js/styles/github.css';
  import {onMount} from "svelte";
  import {invoke} from "@tauri-apps/api/core";
  import {error} from "@fltsci/tauri-plugin-tracing";
  import TreeNav from "$components/help/TreeNav.svelte";

  const md = new MarkdownIt({
    html: true,
    linkify: true,
    typographer: true,
    highlight: (str, lang) => {
      if (lang && hljs.getLanguage(lang)) {
        try {
          return hljs.highlight(str, {language: lang}).value;
        } catch (e) {
          error(e.message);
          return '';
        }
      }
      try {
        return hljs.highlightAuto(str).value;
      } catch (e) {
        error(e.message);
        return '';
      }
    },
  });

  let markdownContent = $state("")

  onMount(async () => {
    let readme = await invoke("help_load_readme", {});
    let license = await invoke("help_load_license", {});
    let changelog = await invoke("help_load_changelog", {});
    let releaseNotes = await invoke("help_load_release_notes", {});
    markdownContent = markdownContent + "\n" + readme + "\n" + license + "\n" + changelog + "\n" + releaseNotes;
  })

  // 渲染
  const rawHtml = $derived(md.render(markdownContent));
  const safeHtml = $derived(DOMPurify.sanitize(rawHtml, {
    USE_PROFILES: {html: true},
    ADD_ATTR: ['target'],
  }));
</script>

<div class="markdown-body" style="display: none">
  {@html safeHtml}
</div>

<div class="app">
  <aside class="sidebar">
    <TreeNav/>
  </aside>
  <main class="content">
  </main>
</div>

<style>
  .markdown-body {
    padding: 1.5rem 2rem;
    font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, 'Helvetica Neue', Arial, sans-serif;
    font-size: 16px;
    line-height: 1.7;
    color: #24292e;
    background: #ffffff;
    max-width: 900px;
    margin: 0 auto;
  }

  /* 标题 */
  .markdown-body :global(h1) {
    font-size: 2.2rem;
    font-weight: 600;
    margin: 1.8rem 0 1rem;
    padding-bottom: 0.3rem;
    border-bottom: 2px solid #eaecef;
  }

  .markdown-body :global(h2) {
    font-size: 1.6rem;
    font-weight: 600;
    margin: 1.5rem 0 0.75rem;
    padding-bottom: 0.3rem;
    border-bottom: 1px solid #eaecef;
  }

  .markdown-body :global(h3) {
    font-size: 1.3rem;
    font-weight: 600;
    margin: 1.2rem 0 0.6rem;
  }

  .markdown-body :global(h4) {
    font-size: 1.1rem;
    font-weight: 600;
    margin: 1rem 0 0.5rem;
  }

  /* 段落 */
  .markdown-body :global(p) {
    margin: 0 0 1rem;
  }

  /* 链接 */
  .markdown-body :global(a) {
    color: #0366d6;
    text-decoration: none;
  }

  .markdown-body :global(a:hover) {
    text-decoration: underline;
  }

  /* 列表 */
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

  /* 引用块 */
  .markdown-body :global(blockquote) {
    margin: 0 0 1rem;
    padding: 0 1rem;
    border-left: 4px solid #dfe2e5;
    color: #6a737d;
  }

  /* 代码块（高亮由 highlight.js 控制） */
  .markdown-body :global(pre) {
    padding: 1rem 1.2rem;
    border-radius: 6px;
    overflow-x: auto;
    margin: 0 0 1rem;
    background: #f6f8fa;
  }

  .markdown-body :global(pre code) {
    font-family: 'SFMono-Regular', Consolas, 'Liberation Mono', Menlo, monospace;
    font-size: 0.9rem;
    line-height: 1.6;
    background: transparent;
    padding: 0;
  }

  /* 行内代码 */
  .markdown-body :global(p code),
  .markdown-body :global(li code) {
    padding: 0.15rem 0.4rem;
    border-radius: 4px;
    background: rgba(27, 31, 35, 0.05);
    font-family: 'SFMono-Regular', Consolas, 'Liberation Mono', Menlo, monospace;
    font-size: 0.9rem;
  }

  /* 表格 */
  .markdown-body :global(table) {
    border-collapse: collapse;
    margin: 0 0 1rem;
    width: 100%;
  }

  .markdown-body :global(th),
  .markdown-body :global(td) {
    padding: 0.5rem 1rem;
    border: 1px solid #dfe2e5;
    text-align: left;
  }

  .markdown-body :global(th) {
    background: #f6f8fa;
    font-weight: 600;
  }

  .markdown-body :global(tr:nth-child(even)) {
    background: #fafbfc;
  }

  /* 分割线 */
  .markdown-body :global(hr) {
    border: 0;
    border-top: 2px solid #eaecef;
    margin: 1.5rem 0;
  }

  /* 图片 */
  .markdown-body :global(img) {
    max-width: 100%;
    height: auto;
    border-radius: 4px;
  }

  /* 强调 */
  .markdown-body :global(strong) {
    font-weight: 600;
  }

  .markdown-body :global(em) {
    font-style: italic;
  }
</style>
