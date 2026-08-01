<script lang="ts">
  import "$styles/components.css";
  import RollcallPage from "$pages/RollcallPage.svelte";
  import RecordHistoryPage from "$pages/RecordHistoryPage.svelte";
  import StudentManagementPage from "$pages/StudentManagementPage.svelte";

  import {ClockCounterClockwiseIcon, DiceFourIcon, UsersIcon} from "phosphor-svelte";

  let currentPage: 'rollcall' | 'students' | 'records' = $state("rollcall");
</script>

<div class="shell">
  <aside class="sidebar">
    <div class="logo">RollCaller</div>
    <nav>
      <button
        class="nav-item"
        class:active={currentPage == "rollcall"}
        onclick={() => currentPage = "rollcall"}
      >
        <DiceFourIcon size="24"/>
      </button>
      <button
        class="nav-item"
        class:active={currentPage == "records"}
        onclick={() => currentPage = "records"}
      >
        <ClockCounterClockwiseIcon size="24"/>
      </button>
      <button
        class="nav-item"
        class:active={currentPage == "students"}
        onclick={() => currentPage = "students"}
      >
        <UsersIcon size="24"/>
      </button>
    </nav>
  </aside>

  <main class="content">
    <div class="page-container">
      <div class="page" class:active={currentPage == "rollcall"}>
        <RollcallPage/>
      </div>
      <div class="page" class:active={currentPage == "records"}>
        <RecordHistoryPage/>
      </div>
      <div class="page" class:active={currentPage == "students"}>
        <StudentManagementPage/>
      </div>
    </div>
  </main>
</div>

<style>
  .shell {
    display: flex;
    height: 100vh;
    width: 100vw;
    overflow: hidden;
    background: #f5f0eb;
    font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, sans-serif;
  }

  .sidebar {
    width: 100px;
    flex-shrink: 0;
    background: #ffffff;
    color: #4a4a4a;
    display: flex;
    flex-direction: column;
    padding: 0;
    border-right: 1px solid #e8e0d8;
    box-shadow: 2px 0 8px rgba(0, 0, 0, 0.04);
  }

  .logo {
    padding: 15px 16px 14px;
    font-size: 14px;
    font-weight: 700;
    color: #8b7355;
    letter-spacing: 0.5px;
    border-bottom: 1px solid #f0ebe5;
    background: #faf8f6;
  }

  nav {
    padding: 12px 8px;
    display: flex;
    flex-direction: column;
    gap: 4px;
  }

  .nav-item {
    display: block;
    width: 100%;
    padding: 10px 12px;
    border: none;
    border-radius: 8px;
    background: transparent;
    color: #6b5e4e;
    cursor: pointer;
    text-align: left;
    font-size: 14px;
    transition: all 0.2s ease;
    font-weight: 500;
  }

  .nav-item:hover {
    background: #f5f0eb;
    color: #5a4a3a;
    transform: translateX(2px);
  }

  .nav-item.active {
    background: #f0e8df;
    color: #7a5a3a;
    font-weight: 600;
    box-shadow: inset 3px 0 0 #c4a88a;
  }

  .content {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    background: #faf8f6;
    position: relative;
    overflow: hidden;
  }

  .page-container {
    position: relative;
    width: 100%;
    height: 100%;
  }

  .page {
    position: absolute;
    top: 0;
    left: 0;
    width: 100%;
    height: 100%;
    overflow-y: auto;
    padding: 24px 32px;

    /* 切换动画 */
    opacity: 0;
    visibility: hidden;
    transition: opacity 0.25s ease, visibility 0s 0.25s;
    pointer-events: none;
    background: #faf8f6;
  }

  .page.active {
    opacity: 1;
    visibility: visible;
    transition: opacity 0.25s ease, visibility 0s 0s;
    pointer-events: auto;
    z-index: 1;
  }

  .page::-webkit-scrollbar {
    width: 6px;
  }

  .page::-webkit-scrollbar-track {
    background: #f5f0eb;
  }

  .page::-webkit-scrollbar-thumb {
    background: #d5c8b8;
    border-radius: 3px;
  }

  .page::-webkit-scrollbar-thumb:hover {
    background: #c4b4a0;
  }
</style>
