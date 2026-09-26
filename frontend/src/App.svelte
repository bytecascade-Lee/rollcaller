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
  import {WindowRouter} from "$routing/windowRouter";
  import {AppCardIds, AppCards, AppEntries, HelpEntries, type AppCard} from "$routing/entries";
  import {type Action, parseRoute} from "$routing/address";

  /** 卡片段的合法性归本窗口：只认 `AppCards` 声明过的 id */
  const isAppCard = (value: string | undefined): value is AppCard =>
    value !== undefined && AppCardIds.some((id) => id === value);

  let APP_INFO = $state<AppInfo>(<AppInfo><unknown>{
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
  let currentPage = $state<AppCard>(AppCards.rollcall);
  let currentAction = $state<Action | null>(null);

  const router = createRouter({
    host: "app",
    handle: (action) => {
      const parsed = parseRoute(action.route);
      const page = parsed?.segments[0];
      if (!isAppCard(page)) {
        return;
      }
      currentPage = page;
      currentAction = action;
      WindowRouter.replaceHash(action.route);
    },
    forward: (action) => NavCommand.navOpen(action),
  });
  WindowRouter.install(router);

  onMount(() => {
    void AppInfoCommand.app_info().then((info) => (APP_INFO = info));

    let disposeRouter: (() => void) | null = null;
    let unlisten: (() => void) | null = null;

    void (async () => {
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
        aria-label="点名"
        class="nav-icon"
        class:active={currentPage == AppCards.rollcall}
        onclick={() => WindowRouter.nav(AppEntries.rollcall)}
        title="点名"
      >
        <DiceFourIcon size="24"/>
      </button>
      <button
        aria-label="学生管理"
        class="nav-icon"
        class:active={currentPage == AppCards.studentManagement}
        onclick={() => WindowRouter.nav(AppEntries.studentManagement)}
        title="学生管理"
      >
        <UsersIcon size="24"/>
      </button>
      <button
        aria-label="历史记录"
        class="nav-icon"
        class:active={currentPage == AppCards.recordHistory}
        onclick={() => WindowRouter.nav(AppEntries.recordHistory)}
        title="历史记录"
      >
        <ClockCounterClockwiseIcon size="24"/>
      </button>
    </nav>
    <nav>
      <button
        aria-label="帮助文档"
        class="nav-icon"
        onclick={() => WindowRouter.nav(HelpEntries.overview)}
        title="帮助文档"
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
    <RollcallPage active={currentPage == AppCards.rollcall}/>
    <RecordHistoryPage active={currentPage == AppCards.recordHistory}/>
    <StudentManagementPage
      action={currentPage == AppCards.studentManagement ? currentAction : null}
      active={currentPage == AppCards.studentManagement}
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
