<script lang="ts">
  import "$styles/global.css";
  import RollcallPage from "$pages/RollcallPage.svelte";
  import RecordHistoryPage from "$pages/RecordHistoryPage.svelte";
  import StudentManagementPage from "$pages/StudentManagementPage.svelte";

  import {ClockCounterClockwiseIcon, DiceFourIcon, UsersIcon} from "phosphor-svelte";

  let currentPage: 'rollcall' | 'students' | 'records' = $state("rollcall");
  const pageNames = {rollcall: "点名", records: "历史记录", students: "学生管理"} as const;
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
  </aside>

  <main class="content">
    <RollcallPage active={currentPage == "rollcall"}/>
    <RecordHistoryPage active={currentPage == "records"}/>
    <StudentManagementPage active={currentPage == "students"}/>
  </main>

  <footer class="footbar">
    <div class="button-group">
      <span class="field-label">当前页面</span>
      <span>{pageNames[currentPage]}</span>
    </div>
    <div class="button-group">
      <span class="footbar-badge">v0.1.0</span>
    </div>
  </footer>
</div>

<style>
  .footbar-badge {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    height: var(--app-size-control-sm);
    padding: 0 var(--app-space-sm);
    border-radius: var(--app-radius-round);
    background: var(--app-color-surface-muted);
    color: var(--app-color-text-muted);
    font-size: var(--app-font-size-xs);
    font-weight: var(--app-font-weight-medium);
  }
</style>
