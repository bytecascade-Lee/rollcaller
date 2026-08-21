<script lang="ts">
  import "open-props/style"
  import "$styles/attendance-status-budge.css"
  import "$styles/button.css";
  import "$styles/card.css";
  import "$styles/content.css";
  import "$styles/popup.css";
  import "$styles/field.css";
  import "$styles/footbar.css";
  import "$styles/global-form-reset.css";
  import "$styles/icon-button.css"
  import "$styles/logo.css"
  import "$styles/nav.css";
  import "$styles/overlay.css";
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
  import {
    ClockCounterClockwiseIcon,
    DiceFourIcon,
    GearIcon,
    ListBulletsIcon,
    QuestionIcon,
    UsersIcon
  } from "phosphor-svelte";
  import {AppInfoCommand, WindowsCommand} from "$commands";
  import type {AppInfo} from "$types";
  import {onMount} from "svelte";
  import TitleBar from "$components/common/TitleBar.svelte";
  import {getCurrentWebviewWindow} from "@tauri-apps/api/webviewWindow";

  let APP_INFO = $state<AppInfo>({
    branch: "",
    commit_count: "",
    short_hash: "",
    commit_time: "",
    version: "",
    build_time: ""
  })
  const window = getCurrentWebviewWindow();
  let currentPage: 'rollcall' | 'students' | 'records' = $state("rollcall");

  onMount(async () => {
    APP_INFO = await AppInfoCommand.app_info();
  });
</script>

<div class="shell">
  <div class="titlebar-slot">
    <TitleBar window={window} title="自动点名应用" label="app"/>
  </div>
  <aside class="sidebar">
    <nav class="nav">
      <button
        class="nav-icon"
        class:active={currentPage == "rollcall"}
        aria-label="点名"
        title="点名"
        onclick={() => currentPage = "rollcall"}
      >
        <DiceFourIcon size="24"/>
      </button>
      <button
        class="nav-icon"
        class:active={currentPage == "students"}
        aria-label="学生管理"
        title="学生管理"
        onclick={() => currentPage = "students"}
      >
        <UsersIcon size="24"/>
      </button>
      <button
        class="nav-icon"
        class:active={currentPage == "records"}
        aria-label="历史记录"
        title="历史记录"
        onclick={() => currentPage = "records"}
      >
        <ClockCounterClockwiseIcon size="24"/>
      </button>
    </nav>
    <nav>
      <button
        class="nav-icon"
        aria-label="帮助文档"
        title="帮助文档"
        onclick={WindowsCommand.openHelpWindow}
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
    <StudentManagementPage active={currentPage == "students"}/>
  </main>

  <footer class="footbar">
    <div>
      <GearIcon size="14" weight="bold" style="display: none"/>
      {APP_INFO.version}+{APP_INFO.branch}.{APP_INFO.commit_count}.{APP_INFO.short_hash}#{APP_INFO.commit_time}
      #{APP_INFO.build_time}
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
