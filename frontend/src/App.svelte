<script lang="ts">
  import "open-props/style"
  import "$styles/attendance-status-budge.css"
  import "$styles/button.css";
  import "$styles/card.css";
  import "$styles/content.css";
  import "$styles/field.css";
  import "$styles/footbar.css";
  import "$styles/global-form-reset.css";
  import "$styles/icon-button.css"
  import "$styles/logo.css"
  import "$styles/nav.css";
  import "$styles/overlay.css";
  import "$styles/popup.css";
  import "$styles/progress.css";
  import "$styles/search.css";
  import "$styles/sidebar.css";
  import "$styles/state.css";
  import "$styles/switch.css";
  import "$styles/table.css";
  import "$styles/text.css";
  import "$styles/tokens.css";
  import "$styles/toolbar.css";
  import RollcallPage from "$pages/RollcallPage.svelte";
  import RecordHistoryPage from "$pages/RecordHistoryPage.svelte";
  import StudentManagementPage from "$pages/StudentManagementPage.svelte";
  import {ClockCounterClockwiseIcon, DiceFourIcon, ListBulletsIcon, QuestionIcon, UsersIcon} from "phosphor-svelte";
  import {AppInfoCommand, NavCommand} from "$commands";
  import type {AppInfo} from "$types";
  import {onMount} from "svelte";
  import AppTitleBar from "$components/app/AppTitlebar.svelte";
  import {getCurrentWebviewWindow} from "@tauri-apps/api/webviewWindow";
  import {listen} from "@tauri-apps/api/event";
  import {createRouter} from "$routing/router";
  import {installWindowRouter, nav} from "$routing/windowRouter";
  import {AppEntries, HelpEntries} from "$routing/entries";
  import {parseRoute, replaceHash, type Action} from "$routing/route";

  /**
   * plane 段 → 卡片。
   *
   * 位置即语法：`#/app/<段>` 的第一段决定挂哪张卡，后面的段属于这张卡内部的叠加层
   * （由卡片自己解释）。存成数组而不是"段 → 组件"的对象字面量，是因为"段合法"
   * 只关心集合，组件引用由模板里的 `{#if}` 决定，两件事不必耦合。
   */
  const PAGES = ["rollcall", "students", "records"] as const;
  type PageKey = (typeof PAGES)[number];
  const isPageKey = (value: string | undefined): value is PageKey =>
    value !== undefined && (PAGES as readonly string[]).includes(value);

  let APP_INFO = $state<AppInfo>({
    os: "",
    arch: "",
    branch: "",
    commit_count: "",
    short_hash: "",
    commit_time: "",
    version: "",
    build_time: ""
  })
  const window = getCurrentWebviewWindow();
  /** 当前卡片：**由地址投影出来**，而不是自己独立持有的视图选择 */
  let currentPage = $state<PageKey>("rollcall");
  /** 最近一次落到本窗口的完整 Action（身份 + 载荷），交给卡片做"初始状态"注入 */
  let currentAction = $state<Action | null>(null);

  const router = createRouter({
    host: "app",
    /** 归本窗口：先把地址翻译成"挂哪张卡"，再把完整 Action 交给那张卡自己解释 */
    handle: (action) => {
      const parsed = parseRoute(action.route);
      const page = parsed?.segments[0];
      if (!isPageKey(page)) return; // 未知段：忽略，不报错也不半应用
      currentPage = page;
      currentAction = action;
      replaceHash(action.route);
    },
    /** 不归本窗口：交给后端投递给目标窗口（跨窗口只有这一条路） */
    forward: (action) => NavCommand.navOpen(action),
  });
  installWindowRouter(router);

  onMount(() => {
    void AppInfoCommand.app_info().then((info) => (APP_INFO = info));

    let disposeRouter: (() => void) | null = null;
    let unlisten: (() => void) | null = null;

    void (async () => {
      // 冷启动：窗口是懒建的，广播必丢，所以载荷留在后端等前端来取
      const pending = await NavCommand.navTakePending();
      const initial = pending
        ? {route: pending.route, payload: pending.payload ?? undefined}
        : undefined;
      disposeRouter = router.start(AppEntries.rollcall, initial);

      unlisten = await listen(NavCommand.NAV_WAKE_EVENT, async () => {
        const next = await NavCommand.navTakePending();
        if (next) router.go({route: next.route, payload: next.payload ?? undefined});
      });
    })();

    return () => {
      disposeRouter?.();
      unlisten?.();
    };
  });
</script>

<div class="shell">
  <div class="titlebar-slot">
    <AppTitleBar window={window}/>
  </div>
  <aside class="sidebar">
    <nav class="nav">
      <button
        class="nav-icon"
        class:active={currentPage == "rollcall"}
        aria-label="点名"
        title="点名"
        onclick={() => nav(AppEntries.rollcall)}
      >
        <DiceFourIcon size="24"/>
      </button>
      <button
        class="nav-icon"
        class:active={currentPage == "students"}
        aria-label="学生管理"
        title="学生管理"
        onclick={() => nav(AppEntries.students)}
      >
        <UsersIcon size="24"/>
      </button>
      <button
        class="nav-icon"
        class:active={currentPage == "records"}
        aria-label="历史记录"
        title="历史记录"
        onclick={() => nav(AppEntries.records)}
      >
        <ClockCounterClockwiseIcon size="24"/>
      </button>
    </nav>
    <nav>
      <button
        class="nav-icon"
        aria-label="帮助文档"
        title="帮助文档"
        onclick={() => nav(HelpEntries.overview)}
      >
        <QuestionIcon size="24"/>
      </button>
      <button
        class="nav-icon"
        style:display="none"
      >
        <ListBulletsIcon size="24"/>
      </button>
    </nav>
  </aside>

  <main class="content">
    <RollcallPage active={currentPage == "rollcall"}/>
    <RecordHistoryPage active={currentPage == "records"}/>
    <StudentManagementPage
      active={currentPage == "students"}
      action={currentPage == "students" ? currentAction : null}
    />
  </main>

  <footer class="footbar">
    <div>
      {APP_INFO.version}+{APP_INFO.branch}.{APP_INFO.commit_count}.{APP_INFO.short_hash}
      #{APP_INFO.commit_time}
      #{APP_INFO.build_time}
    </div>
    <div>
      {APP_INFO.os}-{APP_INFO.arch}
    </div>
  </footer>
</div>

<style>
  .shell {
    display: grid;
    grid-auto-columns: 48px 1fr;
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
</style>
