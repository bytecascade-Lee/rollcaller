<script lang="ts">
  import "open-props"
  import "$styles/attendance-status-budge.css"
  import "$styles/button.css";
  import "$styles/content.css";
  import "$styles/popup.css";
  import "$styles/field.css";
  import "$styles/footbar.css";
  import "$styles/global.css";
  import "$styles/global-form-reset.css";
  import "$styles/icon-button.css"
  import "$styles/nav.css";
  import "$styles/overlay.css";
  import "$styles/search.css";
  import "$styles/sidebar.css";
  import "$styles/state.css";
  import "$styles/table.css";
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
  import {AppInfoCommand} from "$commands";
  import type {AppInfo} from "$types";
  import {onMount} from "svelte";

  let APP_INFO = $state<AppInfo>({branch: "", commit_count: "", short_hash: "", commit_time: "", build_time: ""})
  let currentPage: 'rollcall' | 'students' | 'records' = $state("rollcall");

  onMount(async () => {
    APP_INFO = await AppInfoCommand.app_info();
  });
</script>

<div class="shell">
  <aside class="sidebar">
    <nav class="nav">
      <button
        class="nav-item"
        class:active={currentPage == "rollcall"}
        aria-label="点名"
        title="点名"
        onclick={() => currentPage = "rollcall"}
      >
        <DiceFourIcon size="24"/>
      </button>
      <button
        class="nav-item"
        class:active={currentPage == "students"}
        aria-label="学生管理"
        title="学生管理"
        onclick={() => currentPage = "students"}
      >
        <UsersIcon size="24"/>
      </button>
      <button
        class="nav-item"
        class:active={currentPage == "records"}
        aria-label="历史记录"
        title="历史记录"
        onclick={() => currentPage = "records"}
      >
        <ClockCounterClockwiseIcon size="24"/>
      </button>
    </nav>
    <nav style="display: none">
      <button class="nav-item">
        <QuestionIcon size="24"/>
      </button>
      <button class="nav-item">
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
      v0.1.0+{APP_INFO.branch}.{APP_INFO.commit_count}.{APP_INFO.short_hash}.{APP_INFO.commit_time}->{APP_INFO.build_time}
    </div>
  </footer>
</div>

<style>
  .shell {
    display: grid;
    grid-auto-columns: 56px 1fr;
    grid-template-rows: 1fr auto;
    grid-template-areas:
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
</style>
