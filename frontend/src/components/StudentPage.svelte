<script>

  import {invoke} from "@tauri-apps/api/core";


  /** @type {import('$lib/types').StudentTable[]} */
  let students = $state([]);
  let selectedIds = $state(new Set());
  let searchQuery = $state("");
  let loading = $state(false);

  // Dialog state
  /** @type {'add'|'edit'|'delete'|'export'|null} */
  let dialog = $state(null);
  let editTarget = $state(null); // Student being edited

  // ── Add-dialog inline state ──────────────────
  let addName = $state("");
  let addStudentNo = $state("");
  /** @type {{ type: 'exists'|'conflict'|'restore'|'insert'|'override'|'retain', data: import('$lib/types').StudentTable } | null} */
  let addResult = $state(null); // populated after create attempt

  // ── Derived ──────────────────────────────────────

  let filtered = $derived(
    students.filter(
      (s) =>
        s.name.includes(searchQuery) ||
        s.student_no.includes(searchQuery),
    ),
  );

  let displayList = $derived(filtered.map((s, i) => ({...s, _seq: i + 1})));

  let selectedCount = $derived(selectedIds.size);

  // ── Data helpers ─────────────────────────────────

  // TODO: 后续从前端配置读取用户时区，替换系统时区
  function fmtTs(/** @type {number} */ ms) {
    if (ms == null) return "";
    const d = new Date(ms);
    const pad = (n) => String(n).padStart(2, "0");
    return `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())} ${pad(d.getHours())}:${pad(d.getMinutes())}:${pad(d.getSeconds())}`;
  }

  /** @param {import('$lib/types').StudentTable} s */
  function isSelected(s) {
    return selectedIds.has(s.id);
  }

  /** @param {import('$lib/types').StudentTable} s */
  function toggleSelect(s) {
    const next = new Set(selectedIds);
    if (next.has(s.id)) next.delete(s.id);
    else next.add(s.id);
    selectedIds = next;
  }

  function toggleAll() {
    if (selectedIds.size === filtered.length) {
      selectedIds = new Set();
    } else {
      selectedIds = new Set(filtered.map((s) => s.id));
    }
  }

  // ── Local state mutations ───────────────────────

  /** Add or replace a student in the local list (for insert / restore / override) */
  function upsertStudent(/** @type {import('$lib/types').StudentTable} */ s) {
    const idx = students.findIndex((x) => x.id === s.id);
    if (idx >= 0) {
      // replace in-place via new array to trigger reactivity
      students = [...students.slice(0, idx), s, ...students.slice(idx + 1)];
    } else {
      students = [...students, s];
    }
  }

  /** Remove students by id */
  function removeStudents(/** @type {number[]} */ ids) {
    students = students.filter((s) => !ids.includes(s.id));
    // also clean up selection
    const next = new Set(selectedIds);
    for (const id of ids) next.delete(id);
    selectedIds = next;
  }

  // ── Actions ──────────────────────────────────────

  async function loadStudents() {
    loading = true;
    try {
      students = await invoke("list_all_students");
      selectedIds = new Set();
    } finally {
      loading = false;
    }
  }

  function openAddDialog() {
    addName = "";
    addStudentNo = "";
    addResult = null;
    dialog = "add";
  }

  function openEditDialog(/** @type {import('$lib/types').StudentTable} */ s) {
    const target = s ?? (selectedIds.size === 1
      ? students.find((x) => x.id === [...selectedIds][0])
      : null);
    if (!target) return;
    editTarget = target;
    addName = target.name;
    addStudentNo = target.student_no;
    dialog = "edit";
  }

  async function confirmAdd() {
    if (!addName.trim() || !addStudentNo.trim()) return;
    addResult = null;
    try {
      const res = await invoke("create_student", {
        studentNo: addStudentNo.trim(),
        name: addName.trim(),
      });
      handleCreateResult(res);
    } catch (e) {
      alert(String(e));
    }
  }

  /**
   * @param {import('$lib/types').StudentSingleCreateResult} res
   */
  function handleCreateResult(res) {
    switch (res.type) {
      case "Insert":
      case "Restore":
      case "Override":
        upsertStudent(res.data);
        addResult = {type: res.type.toLowerCase(), data: res.data};
        // 成功后清空输入，方便连续添加
        addName = "";
        addStudentNo = "";
        break;
      case "Retain":
        dialog = null;
        break;
      case "ActiveExists":
        addResult = {type: "exists", data: res.data};
        break;
      case "Conflict":
        addResult = {type: "conflict", data: res.data};
        break;
    }
  }

  async function confirmOverwrite() {
    if (!addName.trim() || !addStudentNo.trim()) return;
    try {
      const res = await invoke("create_student", {
        studentNo: addStudentNo.trim(),
        name: addName.trim(),
        overwrite: true,
      });
      handleCreateResult(res);
    } catch (e) {
      alert(String(e));
    }
  }

  async function confirmEdit() {
    if (!editTarget || !addName.trim() || !addStudentNo.trim()) return;
    try {
      const updated = await invoke("update_student", {
        student: {
          id: editTarget.id,
          student_no: addStudentNo.trim(),
          name: addName.trim(),
        },
      });
      upsertStudent(updated);
      dialog = null;
      editTarget = null;
    } catch (e) {
      alert(String(e));
    }
  }

  function confirmDelete() {
    if (selectedIds.size === 0) return;
    dialog = "delete";
  }

  async function doDelete() {
    const ids = [...selectedIds];
    try {
      await invoke("delete_students", {ids});
      removeStudents(ids);
      dialog = null;
    } catch (e) {
      alert(String(e));
    }
  }

  function confirmExport() {
    if (selectedIds.size > 0) {
      dialog = "export";
    } else {
      doExport("all");
    }
  }

  /**
   * @param {'selected' | 'all'} mode
   */
  async function doExport(mode) {
    dialog = null;
    const data = mode === "selected"
      ? students.filter((s) => selectedIds.has(s.id))
      : filtered;
    console.log("导出数据（待实现）:", data);
    alert("导出功能待 Rust 端实现");
  }


  // 初始加载
  $effect(() => {
    loadStudents();
  });
</script>

<div class="page">
  <!-- ─── Toolbar ─────────────────────────────── -->
  <div class="toolbar">
    <div class="toolbar-button">
      <button onclick={openAddDialog}>+ 添加</button>
      <button onclick={confirmDelete} disabled={selectedCount === 0}>－ 删除</button>
      <button onclick={() => openEditDialog()} disabled={selectedCount !== 1}>✎ 修改</button>
      <button onclick={confirmExport}>↑ 导入</button>
      <button onclick={confirmExport}>↓ 导出</button>
      <button onclick={loadStudents} class="refresh">↻ 刷新</button>
    </div>
    <div class="toolbar-search">
      <input type="search" placeholder="🔍 搜索学号或姓名…" bind:value={searchQuery}/>
    </div>
  </div>

  <!-- ─── Table ───────────────────────────────── -->
  <div class="table-wrap">
    <table>
      <thead>
      <tr>
        <th class="col-cb">
          <input type="checkbox" checked={selectedIds.size === filtered.length && filtered.length > 0}
                 onchange={toggleAll}/>
        </th>
        <th class="col-seq">#</th>
        <th class="col-name">姓名</th>
        <th class="col-no">学号</th>
        <th class="col-ts">创建于</th>
        <th class="col-ts">更新于</th>
      </tr>
      </thead>
      <tbody>
      {#each displayList as s (s.id)}
        <tr class={isSelected(s) ? "selected" : ""}>
          <td class="col-cb">
            <input type="checkbox" checked={isSelected(s)} onchange={() => toggleSelect(s)}/>
          </td>
          <td class="col-seq">{s._seq}</td>
          <td class="col-name">{s.name}</td>
          <td class="col-no">{s.student_no}</td>
          <td class="col-ts">{fmtTs(s.created_at)}</td>
          <td class="col-ts">{fmtTs(s.updated_at)}</td>
        </tr>
      {/each}
      </tbody>
    </table>
    {#if loading}
      <div class="loading">加载中…</div>
    {/if}
    {#if !loading && displayList.length === 0}
      <div class="empty">暂无学生数据</div>
    {/if}
  </div>
</div>

<!-- ─── Dialogs ─────────────────────────────────── -->
{#if dialog === "add"}
  <div class="overlay">
    <div class="dialog">
      <h3>添加学生</h3>
      <label>学号<input type="text" bind:value={addStudentNo} placeholder="如 2024001"/></label>
      <label>姓名<input type="text" bind:value={addName} placeholder="如 张三"/></label>

      <!-- ── Inline result messages ── -->
      {#if addResult?.type === "exists"}
        <div class="msg-box msg-warn">
          <strong>学号已被占用</strong>
          <p>学号「{addResult.data.student_no}」已被学生 <b>{addResult.data.name}</b> 使用<br/>
            （创建于 {fmtTs(addResult.data.created_at)}）。</p>
          <p>请修改学号或姓名后重试。</p>
        </div>
      {:else if addResult?.type === "conflict"}
        <div class="msg-box msg-conflict">
          <strong>学号冲突 — 存在已删除的记录</strong>
          <table class="diff-table">
            <thead>
            <tr>
              <th></th>
              <th>当前输入</th>
              <th>原记录（已删除）</th>
            </tr>
            </thead>
            <tbody>
            <tr>
              <td>学号</td>
              <td>{addStudentNo}</td>
              <td>{addResult.data.student_no}</td>
            </tr>
            <tr>
              <td>姓名</td>
              <td><b>{addName}</b></td>
              <td><b>{addResult.data.name}</b></td>
            </tr>
            </tbody>
          </table>
          <p>原记录已被软删除，是否用新姓名覆盖并恢复？</p>
        </div>
      {:else if addResult?.type === "restore"}
        <div class="msg-box msg-info">
          <strong>已自动恢复</strong>
          <p>学号「{addResult.data.student_no}」曾存在且已删除，系统已自动恢复原记录。</p>
        </div>
      {:else if addResult?.type === "insert"}
        <div class="msg-box msg-success">
          <strong>添加成功</strong>
          <p>学生 <b>{addResult.data.name}</b>（{addResult.data.student_no}）已添加。</p>
        </div>
      {:else if addResult?.type === "override"}
        <div class="msg-box msg-info">
          <strong>已覆写</strong>
          <p>学号「{addResult.data.student_no}」已用新姓名 <b>{addResult.data.name}</b> 覆写并恢复。</p>
        </div>
      {/if}

      <div class="dialog-actions">
        <button class="btn-secondary" onclick={() => dialog = null}>取消</button>
        {#if addResult?.type === "conflict"}
          <button onclick={confirmOverwrite}>覆盖并恢复</button>
        {:else}
          <button onclick={confirmAdd}>确定</button>
        {/if}
      </div>
    </div>
  </div>
{/if}

{#if dialog === "edit"}
  <div class="overlay" onclick={() => dialog = null}>
    <div class="dialog" onclick={(e) => e.stopPropagation()}>
      <h3>修改学生</h3>
      <label>
        学号
        <input type="text" bind:value={addStudentNo}/>
      </label>
      <label>
        姓名
        <input type="text" bind:value={addName}/>
      </label>
      <div class="dialog-actions">
        <button class="btn-secondary" onclick={() => dialog = null}>取消</button>
        <button onclick={confirmEdit}>保存</button>
      </div>
    </div>
  </div>
{/if}

{#if dialog === "delete"}
  <div class="overlay" onclick={() => dialog = null}>
    <div class="dialog" onclick={(e) => e.stopPropagation()}>
      <h3>确认删除</h3>
      <p>确定删除选中的 {selectedCount} 条记录吗？</p>
      <div class="dialog-actions">
        <button class="btn-secondary" onclick={() => dialog = null}>取消</button>
        <button class="btn-danger" onclick={doDelete}>删除</button>
      </div>
    </div>
  </div>
{/if}

{#if dialog === "export"}
  <div class="overlay" onclick={() => dialog = null}>
    <div class="dialog" onclick={(e) => e.stopPropagation()}>
      <h3>导出选项</h3>
      <p>已选中 {selectedCount} 条记录</p>
      <div class="dialog-actions">
        <button class="btn-secondary" onclick={() => dialog = null}>取消</button>
        <button onclick={() => doExport("selected")}>导出选中</button>
        <button onclick={() => doExport("all")}>导出全部</button>
      </div>
    </div>
  </div>
{/if}

<style>
  /* ─── Page layout ─────────────────────────── */
  .page {
    display: flex;
    flex-direction: column;
    height: 100%;
    padding: 16px;
    box-sizing: border-box;
    overflow: hidden;
  }

  /* ─── Toolbar ─────────────────────────────── */
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

  .toolbar-search {
    margin-left: auto;
  }

  .toolbar-search input {
    padding: 6px 10px;
    border: 1px solid #ccc;
    border-radius: 4px;
    font-size: 14px;
    width: 200px;
  }

  button {
    padding: 6px 12px;
    border: 1px solid #bbb;
    border-radius: 4px;
    background: #f8f8f8;
    cursor: pointer;
    font-size: 14px;
  }

  button:hover:not(:disabled) {
    background: #e8e8e8;
  }

  button:disabled {
    opacity: 0.4;
    cursor: not-allowed;
  }

  .refresh {
    background: #e3f2fd;
    border-color: #90caf9;
  }

  /* ─── Table ───────────────────────────────── */
  .table-wrap {
    flex: 1;
    overflow: auto;
    border: 1px solid #ddd;
    border-radius: 4px;
  }

  table {
    width: 100%;
    border-collapse: collapse;
    font-size: 14px;
  }

  thead {
    position: sticky;
    top: 0;
    background: #f5f5f5;
    z-index: 1;
  }

  th,
  td {
    padding: 8px 10px;
    text-align: left;
    border-bottom: 1px solid #eee;
    white-space: nowrap;
  }

  th {
    font-weight: 600;
    color: #444;
  }

  tr.selected {
    background: #e8f4fd;
  }

  tr:hover {
    background: #fafafa;
  }

  .col-cb {
    width: 36px;
    text-align: center;
  }

  .col-seq {
    width: 48px;
    text-align: center;
    color: #888;
  }

  .col-no {
    width: 120px;
  }

  .col-ts {
    width: 150px;
    color: #666;
  }

  .loading,
  .empty {
    padding: 32px;
    text-align: center;
    color: #999;
  }

  /* ─── Dialogs ─────────────────────────────── */
  .overlay {
    position: fixed;
    inset: 0;
    background: rgba(0, 0, 0, 0.35);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 100;
  }

  .dialog {
    background: #fff;
    border-radius: 8px;
    padding: 24px;
    min-width: 320px;
    box-shadow: 0 4px 20px rgba(0, 0, 0, 0.15);
  }

  .dialog h3 {
    margin: 0 0 16px;
    font-size: 16px;
  }

  .dialog label {
    display: block;
    margin-bottom: 12px;
    font-size: 14px;
    color: #555;
  }

  .dialog label input {
    display: block;
    width: 100%;
    margin-top: 4px;
    padding: 6px 8px;
    border: 1px solid #ccc;
    border-radius: 4px;
    font-size: 14px;
    box-sizing: border-box;
  }

  .dialog p {
    margin: 0 0 16px;
    color: #666;
  }

  .dialog-actions {
    display: flex;
    gap: 8px;
    justify-content: flex-end;
    margin-top: 16px;
  }

  .btn-secondary {
    background: #f0f0f0;
  }

  .btn-danger {
    background: #fde8e8;
    border-color: #f5a0a0;
    color: #c00;
  }

  /* ─── Inline message boxes ────────────────── */
  .msg-box {
    margin: 12px 0;
    padding: 10px 14px;
    border-radius: 4px;
    font-size: 13px;
    line-height: 1.5;
    border-left: 4px solid;
  }

  .msg-box strong {
    display: block;
    margin-bottom: 4px;
  }

  .msg-box p {
    margin: 2px 0;
    color: inherit;
  }

  .msg-warn {
    background: #fff3e0;
    border-color: #ff9800;
    color: #663f00;
  }

  .msg-conflict {
    background: #fce4ec;
    border-color: #e53935;
    color: #7b1a1a;
  }

  .msg-info {
    background: #e3f2fd;
    border-color: #1e88e5;
    color: #0d3c5e;
  }

  .msg-success {
    background: #e8f5e9;
    border-color: #43a047;
    color: #1b5e20;
  }

  .diff-table {
    width: 100%;
    border-collapse: collapse;
    margin: 8px 0;
    font-size: 13px;
  }

  .diff-table th,
  .diff-table td {
    padding: 4px 8px;
    border: 1px solid #e0c0c0;
    text-align: left;
  }

  .diff-table th {
    background: #f5d0d0;
    font-weight: 600;
  }
</style>
