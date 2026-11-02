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
  import {nav} from "$routing/nav";
  import {createEntryRegistry} from "$routing/registry";
  import {appDomain} from "$routing/app/domain.svelte";
  import {APP_ENTRY_PARENT, AppEntry, type AppEntryId, type AppEntryPayload} from "$routing/app/entries.generated";
  import {HelpRoute} from "$routing/help/entries.generated";
  import {appNavigate} from "$routing/app/navigate";
  import {DateTimeUtils} from "$utils";

  let APP_INFO = $state<AppInfo | null>(null)
  const window = getCurrentWebviewWindow();

  const overlays = createEntryRegistry<AppEntryId, AppEntryPayload>();
  // 注册表只能由组件创建（它持有组件树），装进域之后"落地"就全在 `appDomain` 里了
  const uninstallLanding = appDomain.install(overlays);
  /** 当前卡片 = 已落地链的链首 */
  const activeCard = $derived(appDomain.chain[0] ?? null);

  const uninstallNav = nav.install({
    domain: appDomain,
    forward: (action) => NavCommand.navOpen(action),
  });

  onMount(() => {
    void AppInfoCommand.app_info().then((info) => (APP_INFO = info));

    let disposeNav: (() => void) | null = null;
    let unlisten: (() => void) | null = null;
    let assertTimer: ReturnType<typeof setTimeout> | null = null;

    void (async () => {
      const pending = await NavCommand.navTakePending();
      const initial = pending
        ? {route: pending.route, payload: pending.payload ?? undefined}
        : undefined;
      disposeNav = nav.start({route: appNavigate.path(AppEntry.rollcall)}, initial);

      unlisten = await listen(NavCommand.NAV_WAKE_EVENT, async () => {
        const next = await NavCommand.navTakePending();
        if (next) nav.go({route: next.route, payload: next.payload ?? undefined});
      });

      // 注册完整性断言：推迟到子组件的 effect 注册之后；根入口（卡片）是平面出口，不需要 handler
      assertTimer = setTimeout(() => {
        const missing = Object.values(AppEntry).filter(
          (id) => APP_ENTRY_PARENT[id] !== null && !overlays.has(id),
        );
        if (missing.length > 0) {
          console.error(`[routing] 以下入口声明了却没有实现：${missing.join(", ")}`);
        }
      }, 0);
    })();

    return () => {
      disposeNav?.();
      unlisten?.();
      uninstallNav();
      uninstallLanding();
      if (assertTimer !== null) clearTimeout(assertTimer);
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
        class:active={activeCard === AppEntry.rollcall}
        onclick={() => appNavigate.enter(AppEntry.rollcall)}
        title="点名"
      >
        <DiceFourIcon size="24"/>
      </button>
      <button
        aria-label="学生管理"
        class="nav-icon"
        class:active={activeCard === AppEntry.studentManagement}
        onclick={() => appNavigate.enter(AppEntry.studentManagement)}
        title="学生管理"
      >
        <UsersIcon size="24"/>
      </button>
      <button
        aria-label="历史记录"
        class="nav-icon"
        class:active={activeCard === AppEntry.recordHistory}
        onclick={() => appNavigate.enter(AppEntry.recordHistory)}
        title="历史记录"
      >
        <ClockCounterClockwiseIcon size="24"/>
      </button>
    </nav>
    <nav>
      <button
        aria-label="帮助文档"
        class="nav-icon"
        onclick={() => nav.go({route: HelpRoute.overview})}
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
    <RollcallPage active={activeCard === AppEntry.rollcall}/>
    <RecordHistoryPage active={activeCard === AppEntry.recordHistory} overlays={overlays}/>
    <StudentManagementPage active={activeCard === AppEntry.studentManagement} overlays={overlays}/>
  </main>

  {#if APP_INFO}
    <footer class="footbar">
      <div>
        {APP_INFO.version}+{APP_INFO.ref_name}.{APP_INFO.commit_count}.{APP_INFO.abbreviated_hash}
        #{DateTimeUtils.format(APP_INFO.commit_time)}
        #{DateTimeUtils.format(APP_INFO.build_time)}
      </div>
      <div>
        {APP_INFO.target}
      </div>
    </footer>
  {/if}
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
