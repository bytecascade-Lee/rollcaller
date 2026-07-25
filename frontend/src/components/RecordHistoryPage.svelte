<script>

  import {invoke} from "@tauri-apps/api/core";


  /** @type {import('$lib/types').RollcallRecord[]} */
  let records = $state([]);
  let searchQuery = $state("");
  let selectedIds = $state(new Set());
  let loading = $state(false);

  // ── 格式化 ──────────────────────────────────────

  function fmtTime(/** @type {number} */ ms) {
    if (ms == null) return "";
    const d = new Date(ms);
    const pad = (n) => String(n).padStart(2, "0");
    return `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())} ${pad(d.getHours())}:${pad(d.getMinutes())}:${pad(d.getSeconds())}`;
  }

  const STATUS_MAP = {
    0: "缺勤",
    1: "出勤",
    2: "迟到",
    3: "早退",
    4: "请假",
  };

  function statusText(/** @type {number} */ code) {
    return STATUS_MAP[code] ?? `未知(${code})`;
  }

  // ── 搜索过滤 ───────────────────────────────────

  let filtered = $derived.by(() => {
    if (!searchQuery.trim()) return records;
    const q = searchQuery.trim().toLowerCase();
    return records.filter((r) => {
      return (
        r.student_no.toLowerCase().includes(q) ||
        r.name.toLowerCase().includes(q) ||
        fmtTime(r.rollcall_at).toLowerCase().includes(q) ||
        r.session_id.toLowerCase().includes(q)
      );
    });
  });

  let displayList = $derived(filtered.map((r, i) => ({...r, _seq: i + 1})));

  let selectedCount = $derived(selectedIds.size);

  // ── 选中逻辑 ───────────────────────────────────

  let allSelected = $derived(
    filtered.length > 0 && selectedIds.size === filtered.length,
  );

  function toggleAll() {
    if (allSelected) {
      selectedIds = new Set();
    } else {
      selectedIds = new Set(filtered.map((r) => r.id));
    }
  }

  function toggleOne(/** @type {number} */ id) {
    const next = new Set(selectedIds);
    if (next.has(id)) next.delete(id);
    else next.add(id);
    selectedIds = next;
  }

  // ── 操作 ───────────────────────────────────────

  async function loadRecords() {
    loading = true;
    try {
      records = await invoke("list_all_records");
      selectedIds = new Set();
    } catch (e) {
      alert("加载记录失败：" + e);
    } finally {
      loading = false;
    }
  }

  function doExport(mode) {
    const data = mode === "selected"
      ? records.filter((r) => selectedIds.has(r.id))
      : filtered;
    console.log("导出数据（待实现）:", data);
    alert("导出功能待实现，控制台可查看数据");
  }

  $effect(() => {
    loadRecords();
  });
</script>

<div class="page">
  <!-- ─── Toolbar ─────────────────────────────── -->
  <div class="toolbar">
    <div class="toolbar-button">
      <button onclick={() => doExport("selected")} disabled={selectedCount === 0}>
        ⇧ 导出选中
      </button>
      <button onclick={() => doExport("all")}>⇧ 导出全部</button>
      <button onclick={loadRecords} class="refresh">↻ 刷新</button>
    </div>
    <div class="toolbar-search">
      <input
        type="search"
        placeholder="🔍 搜索学号、姓名、时间、会话ID…"
        bind:value={searchQuery}
      />
    </div>
  </div>

  <!-- ─── Table ───────────────────────────────── -->
  <div class="table-wrap">
    <table>
      <thead>
      <tr>
        <th class="col-cb">
          <input type="checkbox" checked={allSelected} onchange={toggleAll}/>
        </th>
        <th class="col-seq">#</th>
        <th class="col-name">姓名</th>
        <th class="col-no">学号</th>
        <th class="col-status">状态</th>
        <th class="col-remark">备注</th>
        <th class="col-time">点名时间</th>
        <th class="col-sid">会话ID</th>
      </tr>
      </thead>
      <tbody>
      {#each displayList as r (r.id)}
        <tr>
          <td class="col-cb">
            <input type="checkbox" checked={selectedIds.has(r.id)}
                   onchange={() => toggleOne(r.id)}/>
          </td>
          <td class="col-seq">{r._seq}</td>
          <td class="col-name">{r.name}</td>
          <td class="col-no">{r.student_no}</td>
          <td class="col-status">{statusText(r.attendance_status)}</td>
          <td class="col-remark">{r.remark ?? "—"}</td>
          <td class="col-time">{fmtTime(r.rollcall_at)}</td>
          <td class="col-sid">{r.session_id.slice(0, 8)}…</td>
        </tr>
      {/each}
      {#if !loading && displayList.length === 0}
        <tr>
          <td colspan="8" class="empty">暂无记录</td>
        </tr>
      {/if}
      </tbody>
    </table>
    {#if loading}
      <div class="loading">加载中…</div>
    {/if}
  </div>
</div>

<style>
  .page {
    display: flex;
    flex-direction: column;
    height: 100%;
    padding: 16px;
    box-sizing: border-box;
    overflow: hidden;
  }

  .toolbar {
    display: flex;
    align-items: center;
    gap: 8px;
    margin-bottom: 12px;
    flex-shrink: 0;
  }

  .toolbar-button {
    display: flex;
    gap: 6px;
    flex-wrap: wrap;
  }

  .toolbar-button button {
    padding: 6px 12px;
    border: 1px solid #ced4da;
    border-radius: 5px;
    background: #fff;
    cursor: pointer;
    font-size: 13px;
  }

  .toolbar-button button:hover:not(:disabled) {
    background: #f1f3f5;
  }

  .toolbar-button button:disabled {
    opacity: 0.4;
    cursor: not-allowed;
  }

  .toolbar-button button.refresh {
    background: #e8f4fd;
    border-color: #b6d4fe;
  }

  .toolbar-search {
    margin-left: auto;
  }

  .toolbar-search input[type="search"] {
    padding: 6px 10px;
    border: 1px solid #ced4da;
    border-radius: 5px;
    font-size: 13px;
    width: 200px;
  }

  .table-wrap {
    flex: 1;
    overflow: auto;
    border: 1px solid #e9ecef;
    border-radius: 6px;
    position: relative;
  }

  table {
    width: 100%;
    border-collapse: collapse;
    font-size: 13px;
  }

  th {
    background: #f8f9fa;
    padding: 8px 6px;
    text-align: left;
    font-weight: 600;
    color: #495057;
    border-bottom: 2px solid #dee2e6;
    position: sticky;
    top: 0;
    z-index: 1;
  }

  td {
    padding: 6px;
    border-bottom: 1px solid #e9ecef;
    color: #2c3e50;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  tbody tr:hover {
    background: #f8f9fa;
  }

  .col-cb {
    width: 36px;
    text-align: center;
  }

  .col-cb input {
    margin: 0;
  }

  .col-seq {
    width: 40px;
    text-align: center;
    color: #888;
  }

  .col-name {
    width: 80px;
  }

  .col-no {
    width: 100px;
  }

  .col-status {
    width: 60px;
  }

  .col-remark {
    min-width: 80px;
  }

  .col-time {
    width: 150px;
    color: #666;
  }

  .col-sid {
    width: 80px;
    font-family: monospace;
    font-size: 12px;
    color: #6c757d;
  }

  .loading {
    position: absolute;
    inset: 0;
    display: flex;
    align-items: center;
    justify-content: center;
    background: rgba(255, 255, 255, 0.7);
    color: #999;
    font-size: 14px;
  }

  .empty {
    text-align: center;
    color: #aaa;
    padding: 40px 0 !important;
  }
</style>
