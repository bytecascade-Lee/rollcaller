<script lang="ts">
  import "open-props/style"
  import "highlight.js/styles/github.css";
  import "$styles/tokens.css";
  import "$styles/sidebar.css";
  import "$styles/nav.css";
  import "$styles/content.css";
  import "$styles/footbar.css";
  import "$styles/text.css";
  import "$styles/global-form-reset.css"
  import "$styles/popup.css"
  import "$styles/search.css"
  import "$styles/state.css"
  import {onMount} from "svelte";
  import TitleBar from "$components/common/TitleBar.svelte";
  import {getCurrentWebviewWindow} from "@tauri-apps/api/webviewWindow";
  import NavTree from "$components/common/NavTree.svelte";
  import MarkdownView from "$components/common/MarkdownView.svelte";
  import {helpStore} from "$stores/helpStore.svelte";
  import {AppInfoCommand} from "$commands";
  import type {AppInfo, TreeNode} from "$types";
  import {GearIcon} from "phosphor-svelte";
  import metaData from "$resources/help/meta.json";

  const nodes = metaData.nodes as
    {
      [K in keyof typeof metaData.nodes]: TreeNode & { id: K }
    };
  const window = getCurrentWebviewWindow();
  let activeId = $state("overview");
  let jumpToken = $state(0); // 外部跳转触发信号：自增时 NavTree 折叠到单级
  let history = $state<string[]>(["overview"]);
  let historyIndex = $state(0);
  let canGoBack = $derived(historyIndex > 0);
  let canGoForward = $derived(historyIndex < history.length - 1);
  let scroller: HTMLDivElement | undefined = $state();
  /** 待滚动的锚点名（普通变量，不参与响应式）：锚点跳转由 handleNavigate 的续段负责 */
  let pendingSection: string | null = null;
  let APP_INFO = $state<AppInfo>({
    branch: "",
    commit_count: "",
    short_hash: "",
    commit_time: "",
    version: "",
    build_time: ""
  });

  /** 内部链接跳转：加载文档，并让左侧树跳转到对应节点（折叠到单级）；带锚点时滚动到目标章节 */
  async function handleNavigate(id: string, link?: string) {
    activeId = id;
    jumpToken += 1;
    await loadDoc(id, link);
    pushHistory(id);
  }

  /** 侧边栏树点击：加载文档（不折叠树） */
  async function handleSidebarSelect(id: string) {
    activeId = id;
    await helpStore.load(id);
    pushHistory(id);
  }

  /** 加载文档并处理锚点滚动 */
  async function loadDoc(id: string, link?: string | null) {
    const target = link ?? null;
    pendingSection = target;
    await helpStore.load(id);
    if (pendingSection !== target) return;
    scrollToSection(target);
    pendingSection = null;
  }

  /** 追加历史记录，裁剪前进栈 */
  function pushHistory(id: string) {
    const trimmed = history.slice(0, historyIndex + 1);
    history = [...trimmed, id];
    historyIndex = history.length - 1;
  }

  /** 后退到上一条历史记录（不记录新历史） */
  async function goBack() {
    if (!canGoBack) return;
    historyIndex -= 1;
    const id = history[historyIndex];
    activeId = id;
    jumpToken += 1;
    await loadDoc(id);
  }

  /** 前进到下一条历史记录（不记录新历史） */
  async function goForward() {
    if (!canGoForward) return;
    historyIndex += 1;
    const id = history[historyIndex];
    activeId = id;
    jumpToken += 1;
    await loadDoc(id);
  }

  /** 滚动到锚点标题（缺失时回退到顶部）。滚动容器是 .content > .active，window 不参与滚动 */
  function scrollToSection(section: string | null) {
    if (!scroller) return;
    if (section) {
      // 只匹配标题上的 data-section（链接不再携带该属性，避免命中目录项自身）
      const el = scroller.querySelector(`[data-section="${section}"]`);
      if (el) {
        const top = el.getBoundingClientRect().top - scroller.getBoundingClientRect().top + scroller.scrollTop;
        scroller.scrollTo({top, behavior: "smooth"});
        return;
      }
    }
    scroller.scrollTo(0, 0);
  }

  // 切换文档后滚动条回到顶部（锚点跳转在途时跳过，避免与平滑滚动打架）
  $effect(() => {
    helpStore.content;
    if (pendingSection) return;
    scroller?.scrollTo(0, 0);
  });

  onMount(() => {
    // 注册跳转回调：搜索结果选中时复用导航逻辑（加载 + 树高亮 + 折叠）
    helpStore.navigate = handleNavigate;
    void (async () => {
      APP_INFO = await AppInfoCommand.app_info();
      await helpStore.load(activeId);
    })();
    return () => {
      helpStore.navigate = null;
    };
  });
</script>

<div class="shell">
  <div class="titlebar-slot">
    <TitleBar window={window} title="自动点名 - 帮助文档" label={window.label}
              onback={goBack} onforward={goForward} canGoBack={canGoBack} canGoForward={canGoForward}/>
  </div>
  <aside class="sidebar">
    <nav class="nav">
      <NavTree
        bind:activeId={activeId}
        jumpToken={jumpToken}
        nodes={nodes}
        onselect={handleSidebarSelect}
        order={metaData.order}
      />
    </nav>
  </aside>

  <main class="content">
    {#if helpStore.content}
      <div class="active" bind:this={scroller}>
        <MarkdownView markdown={helpStore.content} docId={activeId} onnavigate={handleNavigate}/>
      </div>
    {:else}
      <div class="empty active">
        <p class="text-content">当前节点下暂无内容</p>
      </div>
    {/if}
  </main>

  <footer class="footbar">
    <div>
      <GearIcon size="14" style="display: none" weight="bold"/>
      {APP_INFO.version}+{APP_INFO.branch}.{APP_INFO.commit_count}.{APP_INFO.short_hash}#{APP_INFO.commit_time}
      #{APP_INFO.build_time}
    </div>
  </footer>
</div>

<style>
  .shell {
    display: grid;
    grid-auto-columns: 130px 1fr;
    grid-template-rows: auto 1fr auto;
    grid-template-areas:
      "titlebar titlebar"
      "sidebar content"
      "footbar footbar";
    width: 100%;
    height: 100%;
    overflow: hidden;
    background: var(--color-page);
    color: var(--text-color-content);
    font-family: var(--font-family-sans);
    font-size: var(--font-size-sm);
  }

  .titlebar-slot {
    grid-area: titlebar;
  }

  .sidebar {
    min-height: 0;
    overflow: auto;
  }

  .empty {
    align-items: center;
    justify-content: center;
    color: var(--text-color-secondary);
  }

  .content > .active {
    overflow-y: auto;
  }
</style>
