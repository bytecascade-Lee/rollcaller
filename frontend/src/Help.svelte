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
  import {getCurrentWebviewWindow} from "@tauri-apps/api/webviewWindow";
  import {listen} from "@tauri-apps/api/event";
  import {NavCommand} from "$commands";
  import NavTree from "$components/common/NavTree.svelte";
  import MarkdownView from "$components/common/MarkdownView.svelte";
  import {helpStore} from "$stores/helpStore.svelte";
  import type {TreeNode} from "$types";
  import metaData from "$resources/help/meta.json";
  import HelpTitlebar from "$components/help/HelpTitlebar.svelte";
  import {HelpAddress} from "$utils/helpAddress";
  import {CaretRightIcon, CircleIcon} from "phosphor-svelte";
  import {createRouter} from "$routing/router";
  import {installWindowRouter, nav} from "$routing/windowRouter";
  import {HelpEntries} from "$routing/entries";
  import {replaceHash} from "$routing/route";
  import {createOverlayRegistry} from "$routing/overlays";
  import type {HelpOverlayName} from "$routing/overlayNames";

  const nodes = metaData.nodes as
    {
      [K in keyof typeof metaData.nodes]: TreeNode & { id: K }
    };
  const window = getCurrentWebviewWindow();
  const overlays = createOverlayRegistry<HelpOverlayName>();
  let activeId = $state(HelpAddress.DEFAULT_DOC);
  let jumpToken = $state(0); // 外部跳转触发信号：自增时 NavTree 折叠到单级
  /** 历史栈初始为空：地址由 `router.start()` 的第一条投递立起来，避免"首帧与地址不一致" */
  let history = $state<HelpAddress[]>([]);
  let historyIndex = $state(0);
  let canGoBack = $derived(historyIndex > 0);
  let canGoForward = $derived(historyIndex < history.length - 1);
  let scroller: HTMLDivElement | undefined = $state();
  /** 待滚动的锚点名（普通变量，不参与响应式）：锚点跳转由 applyAddress 的续段负责 */
  let pendingSection: string | null = null;
  /** 当前地址（历史栈顶），用于幂等判定 */
  const currentAddress = $derived(history[historyIndex]);
  /** 从 activeId 沿 parentId 回溯，生成面包屑路径（根→当前） */
  let breadcrumbCrumb = $derived.by(() => {
    const trail: string[] = [];
    let cur: string | null = activeId;
    while (cur && cur in nodes) {
      const node = nodes[cur as keyof typeof nodes] as TreeNode;
      trail.push(node.title);
      cur = node.parentId;
    }
    trail.reverse();
    return trail;
  });

  const router = createRouter({
    host: HelpAddress.HOST,
    handle: (action) => {
      const address = HelpAddress.parse(action.route);
      if (address) void applyAddress(address);
    },
  });
  installWindowRouter(router);

  /**
   * 本窗口的落地函数：把地址应用到当前视图，并把地址写回 `location.hash`。
   *
   * hash 只是**对外投影**（刷新、原生 `<a href="#/...">`、调试用），真源仍是地址本身；
   * 写它用 `replaceState`（见 `replaceHash`），所以不会与这里自有的前进/后退栈打架。
   *
   * - `foldTree`：外部跳转（文档内链接 / 搜索）折叠导航树到当前路径，侧栏点击不折叠
   * - 重复投递同一地址是空操作，只重新定位（避免连点把历史栈灌满）
   */
  async function applyAddress(address: HelpAddress, foldTree = true) {
    const hash = address.toHash();
    replaceHash(hash);
    if (hash === currentAddress?.toHash()) {
      scrollToSection(address.anchor);
      return;
    }
    activeId = address.doc;
    if (foldTree) jumpToken += 1;
    await loadDoc(address);
    pushHistory(address);
  }

  /** 侧边栏树点击：加载文档（不折叠树） */
  async function handleSidebarSelect(id: string) {
    await applyAddress(HelpAddress.of(id), false);
  }

  /** 加载文档并处理锚点滚动 */
  async function loadDoc(address: HelpAddress) {
    const target = address.anchor;
    pendingSection = target;
    await helpStore.load(address.doc);
    if (pendingSection !== target) return;
    scrollToSection(target);
    pendingSection = null;
  }

  /** 追加历史记录，裁剪前进栈 */
  function pushHistory(address: HelpAddress) {
    const trimmed = history.slice(0, historyIndex + 1);
    history = [...trimmed, address];
    historyIndex = history.length - 1;
  }

  /** 后退到上一条历史记录（不记录新历史，锚点一并还原） */
  async function goBack() {
    if (!canGoBack) return;
    historyIndex -= 1;
    const address = history[historyIndex];
    activeId = address.doc;
    jumpToken += 1;
    replaceHash(address.toHash());
    await loadDoc(address);
  }

  /** 前进到下一条历史记录（不记录新历史，锚点一并还原） */
  async function goForward() {
    if (!canGoForward) return;
    historyIndex += 1;
    const address = history[historyIndex];
    activeId = address.doc;
    jumpToken += 1;
    replaceHash(address.toHash());
    await loadDoc(address);
  }

  /** 滚动到锚点标题（缺失时回退到顶部）。滚动容器是 .content > .active，window 不参与滚动 */
  function scrollToSection(section: string | null) {
    if (!scroller) return;
    if (section) {
      // 锚点即标题上的 DOM id（源文 `## 标题 {#section}` 经 headingIdPlugin 落到 id 属性）
      const el = scroller.querySelector(`[id="${section}"]`);
      if (el) {
        const top = el.getBoundingClientRect().top - scroller.getBoundingClientRect().top + scroller.scrollTop;
        scroller.scrollTo({top, behavior: "smooth"});
        return;
      }
    }
    // 容器 CSS 已设 smooth，回退顶部需显式 instant，避免整页缓慢滚动
    scroller.scrollTo({top: 0, behavior: "instant"});
  }

  // 切换文档后滚动条回到顶部（锚点跳转在途时跳过，避免与平滑滚动打架）
  $effect(() => {
    helpStore.content;
    if (pendingSection) return;
    scroller?.scrollTo({top: 0, behavior: "instant"});
  });

  onMount(() => {
    // 注册跳转回调：搜索结果选中、文档内链接都经 router 走同一个门
    helpStore.navigate = (address) => nav({route: address.toHash()});

    let dispose: (() => void) | null = null;
    let unlisten: (() => void) | null = null;

    void (async () => {
      // 冷启动：本窗口是懒建的，建窗那一刻前端还没开始监听，广播必丢 ⇒
      // 载荷留在后端等这里来取，所以"取走"这一步同时就是"我起来了"的信号
      const pending = await NavCommand.navTakePending();
      const initial = pending
        ? {route: pending.route, payload: pending.payload ?? undefined}
        : undefined;
      // 首帧落点：有冷启动载荷就用它，否则当前 hash 若是本窗口的地址就用它，再否则回概览
      dispose = router.start(HelpEntries.overview, initial);

      unlisten = await listen(NavCommand.NAV_WAKE_EVENT, async () => {
        const next = await NavCommand.navTakePending();
        if (next) nav({route: next.route, payload: next.payload ?? undefined});
      });
    })();

    return () => {
      dispose?.();
      unlisten?.();
      helpStore.navigate = null;
    };
  });
</script>

<div class="shell">
  <div class="titlebar-slot">
    <HelpTitlebar window={window} {overlays}
                  onback={goBack} onforward={goForward}
                  canGoBack={canGoBack} canGoForward={canGoForward}
    />
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
        <MarkdownView markdown={helpStore.content} onnavigate={applyAddress}/>
      </div>
    {:else}
      <div class="empty active">
        <p class="text-content">当前节点下暂无内容</p>
      </div>
    {/if}
  </main>

  <footer class="footbar">
    <div>
      <CircleIcon size="7" weight="fill"/>
      {#each breadcrumbCrumb as crumb, i}
        {#if i > 0}
          <CaretRightIcon size="7" weight="bold"/>
        {/if}
        <span>
          {crumb}
        </span>
      {/each}
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

  /* 同文档锚点（`#xxx`）不拦截，交浏览器原生滚动；平滑由这里给 */
  .content > .active {
    overflow-y: auto;
    scroll-behavior: smooth;
  }
</style>
