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
  import {createEntryRegistry} from "$routing/registry";
  import {appAction, appPath, chainOf, HelpEntries, resolveAppEntry} from "$routing/entries";
  import {type AppEntryId, AppEntry, APP_ENTRY_PARENT} from "$routing/entries.generated";
  import {type Payload, parseRoute} from "$routing/address";

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

  /** 本窗口的落点注册表：**只有叠加层注册**（平面的呈现见下面的 `activeChain` 投影） */
  const overlays = createEntryRegistry<AppEntryId>();

  /** 当前地址对应的入口链（根 → 目标）。平面的呈现 = 链首，故卡片不需要 handler */
  let activeChain = $state<AppEntryId[]>([]);
  const activeCard = $derived(activeChain[0] ?? null);

  /**
   * 转换语义：按新旧链的**共同前缀** diff。
   *
   * 旧的余段逆序 `close`，新的余段顺序 `open`，载荷只交给最深的那一个（祖先只需被呈现）。
   * 与 `open` 对称，且不会把共有的祖先（例如卡片）关掉再开回来 —— 从 `edit-student`
   * 切到 `delete-student` 时卡片纹丝不动。
   *
   * **整条链都相同（重放同一地址）时要单独再 `open` 一次目标**，否则会静默失效：
   * 见下面 `shared === next.length` 那一支的注释。
   */
  function transition(entry: AppEntryId, payload?: Payload) {
    const next = chainOf(entry);
    const prev = activeChain;
    let shared = 0;
    while (shared < prev.length && shared < next.length && prev[shared] === next[shared]) {
      shared += 1;
    }
    for (let i = prev.length - 1; i >= shared; i -= 1) {
      overlays.close(prev[i]);
    }
    for (let i = shared; i < next.length; i += 1) {
      overlays.open(next[i], i === next.length - 1 ? payload : undefined);
    }
    // 重放同一地址：diff 会判定"无事可做"，但目标此刻未必真的开着 ——
    // 关闭**不经过地址**（用户点"取消"不会让地址回退），所以"地址停在 delete-student
    // 而弹窗早已关闭"是常见状态。按 §8「同 route 重放 = 重新应用初始状态」，这里补一次
    // 目标的 open（载荷一并重放）。
    if (shared === next.length) {
      overlays.open(entry, payload);
    }
    activeChain = next;
  }

  const router = createRouter({
    host: "app",
    handle: (action) => {
      const segments = parseRoute(action.route)?.segments ?? [];
      const entry = resolveAppEntry(segments);
      if (!entry) {
        // 降级：地址非法（入口名不认识，或父链与声明的 parentId 不一致）→ 原地不动
        console.error(`[routing] 非法 app 地址：${action.route}`);
        return;
      }
      transition(entry, action.payload);
      WindowRouter.replaceHash(action.route);
    },
    forward: (action) => NavCommand.navOpen(action),
  });
  WindowRouter.install(router);

  onMount(() => {
    void AppInfoCommand.app_info().then((info) => (APP_INFO = info));

    let disposeRouter: (() => void) | null = null;
    let unlisten: (() => void) | null = null;
    let assertTimer: ReturnType<typeof setTimeout> | null = null;

    void (async () => {
      const pending = await NavCommand.navTakePending();
      const initial = pending
        ? {route: pending.route, payload: pending.payload ?? undefined}
        : undefined;
      disposeRouter = router.start({route: appPath(AppEntry.rollcall)}, initial);

      unlisten = await listen(NavCommand.NAV_WAKE_EVENT, async () => {
        const next = await NavCommand.navTakePending();
        if (next) router.go({route: next.route, payload: next.payload ?? undefined});
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
      disposeRouter?.();
      unlisten?.();
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
        onclick={() => WindowRouter.nav(appAction(AppEntry.rollcall))}
        title="点名"
      >
        <DiceFourIcon size="24"/>
      </button>
      <button
        aria-label="学生管理"
        class="nav-icon"
        class:active={activeCard === AppEntry.studentManagement}
        onclick={() => WindowRouter.nav(appAction(AppEntry.studentManagement))}
        title="学生管理"
      >
        <UsersIcon size="24"/>
      </button>
      <button
        aria-label="历史记录"
        class="nav-icon"
        class:active={activeCard === AppEntry.recordHistory}
        onclick={() => WindowRouter.nav(appAction(AppEntry.recordHistory))}
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
    <RollcallPage active={activeCard === AppEntry.rollcall}/>
    <RecordHistoryPage active={activeCard === AppEntry.recordHistory} overlays={overlays}/>
    <StudentManagementPage active={activeCard === AppEntry.studentManagement} overlays={overlays}/>
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
