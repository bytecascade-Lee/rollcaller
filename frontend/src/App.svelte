<script lang="ts">
  import "$styles/components.css";
  import RollcallPage from "$components/RollcallPage.svelte";
  import RecordHistoryPage from "$pages/RecordHistoryPage.svelte";
  import StudentManagementPage from "$pages/StudentManagementPage.svelte";

  let currentPage: 'rollcall' | 'students' | 'records' = $state("rollcall");
</script>

<div class="shell">
  <aside class="sidebar">
    <div class="logo">RollCaller</div>
    <nav>
      <button class="nav-item" class:active={currentPage == "rollcall"} onclick={() => currentPage = "rollcall"}>
        🎲 自动点名
      </button>
      <button
        class="nav-item"
        class:active={currentPage == "records"}
        onclick={() => currentPage = "records"}
      >
        📋 历史记录
      </button>
      <button
        class="nav-item"
        class:active={currentPage == "students"}
        onclick={() => currentPage = "students"}
      >
        👤 学生管理
      </button>
    </nav>
  </aside>
  <main class="content">
    {#if currentPage == "rollcall"}
      <RollcallPage />
    {:else if currentPage == "records"}
      <RecordHistoryPage />
    {:else if currentPage == "students"}
      <StudentManagementPage />
    {/if}
  </main>
</div>

<style>
  .shell {
    display: flex;
    height: 100vh;
    width: 100vw;
    overflow: hidden;
  }

  .sidebar {
    width: 120px;
    flex-shrink: 0;
    background: #2c3e50;
    color: #ecf0f1;
    display: flex;
    flex-direction: column;
    padding: 0;
  }

  .logo {
    padding: 16px 12px 10px;
    font-size: 15px;
    font-weight: 700;
    border-bottom: 1px solid rgba(255, 255, 255, 0.1);
  }

  nav {
    padding: 6px;
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .nav-item {
    display: block;
    width: 100%;
    padding: 8px 10px;
    border: none;
    border-radius: 5px;
    background: transparent;
    color: #bdc3c7;
    cursor: pointer;
    text-align: left;
    font-size: 13px;
    transition: background 0.15s, color 0.15s;
  }

  .nav-item:hover {
    background: rgba(255, 255, 255, 0.08);
    color: #fff;
  }

  .nav-item.active {
    background: rgba(255, 255, 255, 0.12);
    color: #fff;
    font-weight: 600;
  }

  .content {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    background: #fff;
    position: relative;
    overflow: hidden;
  }
</style>