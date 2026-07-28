<script lang="ts">

  import {isLoading, load, records, select, selectAll, selected} from "$stores/recordStore.svelte";
  import {format} from "$utils/DataTimeUtils";
  import type {RollcallRecord} from "$types/RollcallRecord";
  import {FileUp, RotateCw} from "@o7/icon/lucide";
  import {statusText} from "$constants/AttendanceStatus";

  let searchQuery = $state("");

  let display = $derived.by<RollcallRecord[]>(() => {
    if (!searchQuery.trim()) return records;
    const q = searchQuery.trim().toLowerCase();
    return records.filter((r) => {
      return (
        r.student_no.toLowerCase().includes(q) ||
        r.name.toLowerCase().includes(q) ||
        format(r.rollcall_at).toLowerCase().includes(q)
      );
    });
  });

  let displaySelectedCount = $derived(display.filter(r => selected.has(r.id)).length)

  $effect(() => {
    load()
  });

</script>

<div>
  <div>
    <div>
      <button onclick={() => alert("selected")} disabled={selected.size == 0}>
        <FileUp/>
        导出选中
      </button>
      <button onclick={() => alert("all")}>
        <FileUp/>
        导出全部
      </button>
      <button onclick={load}>
        <RotateCw/>
        刷新
      </button>
    </div>
    <div class="toolbar-search">
      <input
        type="search"
        placeholder="🔍 搜索姓名、学号和点名时间"
        bind:value={searchQuery}
      />
    </div>
  </div>

  <div>
    <table>
      <thead>
      <tr>
        <th><input
          type="checkbox"
          checked={display.length > 0 && displaySelectedCount == display.length}
          indeterminate={displaySelectedCount > 0 && displaySelectedCount < display.length}
          onchange={selectAll}/></th>
        <th>序号</th>
        <th>姓名</th>
        <th>学号</th>
        <th>状态</th>
        <th>备注</th>
        <th>点名时间</th>
        <th>session id</th>
      </tr>
      </thead>
      <tbody>
      {#if (isLoading)}
        <tr>
          <td colspan="5">数据加载中...</td>
        </tr>
      {:else if records.length === 0}
        <tr>
          <td colspan="5">暂无历史记录</td>
        </tr>
      {:else}
        {#each display as record, index (record.id)}
          <tr>
            <td>
              <input
                type="checkbox"
                checked={selected.has(record.id)}
                onchange={() => select(record.id)}
              />
            </td>
            <td>{index + 1}</td>
            <td>{record.name}</td>
            <td>{record.student_no}</td>
            <td>{statusText(record.attendance_status)}</td>
            <td>{record.remark}</td>
            <td>{format(record.rollcall_at)}</td>
            <td>{record.session_id}</td>
          </tr>
        {/each}
      {/if}
      </tbody>
    </table>
  </div>
</div>
