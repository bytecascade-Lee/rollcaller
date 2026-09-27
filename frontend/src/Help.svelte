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
  import type {HelpOverlayName, TreeNode} from "$types";
  import metaData from "$resources/help/meta.json";
  import HelpTitlebar from "$components/help/HelpTitlebar.svelte";
  import {HelpAddress} from "$utils/helpAddress";
  import {CaretRightIcon, CircleIcon} from "phosphor-svelte";
  import {createRouter} from "$routing/router";
  import {WindowRouter} from "$routing/windowRouter";
  import {HelpEntry} from "$routing/entries.generated";
  import {createEntryRegistry} from "$routing/registry";
  import {helpNavigate} from "$routing/helpNavigate.svelte";

  const nodes = metaData.nodes as
    {
      [K in keyof typeof metaData.nodes]: TreeNode & { id: K }
    };
  const window = getCurrentWebviewWindow();
  const overlays = createEntryRegistry<HelpOverlayName>();
  /** 外部跳转触发信号：自增时 NavTree 折叠到单级 */
  let jumpToken = $state(0);
  let scroller: HTMLDivElement | undefined = $state();
  /** 当前文档 = 导航屏幕上那一篇；树高亮与面包屑都从它派生 */
  const activeId = $derived(helpNavigate.current?.doc ?? HelpAddress.DEFAULT_DOC);
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
      if (address) void helpNavigate.apply(address);
    },
    /** 不归本窗口：交给后端投递给目标窗口（与主窗口同一条路） */
    forward: (action) => NavCommand.navOpen(action),
  });
  WindowRouter.install(router);

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
    if (helpNavigate.pendingAnchor) return;
    scroller?.scrollTo({top: 0, behavior: "instant"});
  });

  onMount(() => {
    // 交出滚动容器与折叠树：落点归导航，只有这两件视图事留在窗口
    const uninstall = helpNavigate.install({
      scrollTo: scrollToSection,
      foldTree: () => (jumpToken += 1),
    });

    // 注册跳转回调：搜索结果选中、文档内链接都经导航走同一个门
    helpStore.navigate = (address) => helpNavigate.open(address);

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
      dispose = router.start({route: HelpEntry.overview}, initial);

      unlisten = await listen(NavCommand.NAV_WAKE_EVENT, async () => {
        const next = await NavCommand.navTakePending();
        if (next) WindowRouter.nav({route: next.route, payload: next.payload ?? undefined});
      });
    })();

    return () => {
      dispose?.();
      unlisten?.();
      uninstall();
      helpStore.navigate = null;
    };
  });
</script>

<div class="shell">
  <div class="titlebar-slot">
    <HelpTitlebar canGoBack={helpNavigate.canGoBack} canGoForward={helpNavigate.canGoForward}
                  onback={() => void helpNavigate.back()} onforward={() => void helpNavigate.forward()}
                  {overlays} window={window}
    />
  </div>
  <aside class="sidebar">
    <nav class="nav">
      <NavTree
        activeId={activeId}
        jumpToken={jumpToken}
        nodes={nodes}
        onselect={(id) => helpNavigate.open(HelpAddress.of(id), {fold: false})}
        order={metaData.order}
      />
    </nav>
  </aside>

  <main class="content">
    {#if helpStore.content}
      <div class="active" bind:this={scroller}>
        <MarkdownView markdown={helpStore.content}
                      onnavigate={(address) => helpNavigate.open(address)}/>
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
