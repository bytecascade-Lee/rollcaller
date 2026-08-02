<script lang="ts">

  import {recordStore} from "$stores/recordStore.svelte";
  import {format} from "$utils/DataTimeUtils";
  import type {RecordGroupMetaData, RollcallRecord} from "$types";
  import {ArrowClockwiseIcon, FileArrowDownIcon, MagnifyingGlassIcon, PencilIcon, PencilSimpleIcon} from "phosphor-svelte"
  import {COLORS, group} from "$services/RecordService.svelte";
  import AttendanceStatusBadge from "$components/record-history/AttendanceStatusBadge.svelte";
  import EditRecord from "$components/record-history/EditRecord.svelte";
  import {overlayController} from "$controllers/overlayController";

  let selected = $state<Set<bigint>>(new Set());
  let {active = $bindable(false)} = $props();
  let searchQuery = $state("");
  let anchor = $state<HTMLElement | null>(null);

  let display = $derived.by<RollcallRecord[]>(() => {
    if (!searchQuery.trim()) return recordStore.records;
    const q = searchQuery.trim().toLowerCase();
    return recordStore.records.filter((r) => {
      return (
        r.student_no.toLowerCase().includes(q) ||
        r.name.toLowerCase().includes(q) ||
        r.remark?.toLowerCase().includes(q) ||
        format(r.rollcall_at).toLowerCase().includes(q)
      );
    });
  });

  let groupInfo = $derived<RecordGroupMetaData[]>(group(display));
  let displaySelectedCount = $derived(display.filter(r => selected.has(r.id)).length)

  export function select(id: bigint) {
    if (selected.has(id)) {
      let set = new Set(selected);
      set.delete(id)
      selected = set;
    } else {
      selected = new Set([...selected, id]);
    }
  }

  export function selectAll() {
    if (selected.size == recordStore.records.length) {
      selected = new Set<bigint>();
    } else {
      let set = new Set<bigint>();
      for (let record of recordStore.records) {
        set.add(record.id);
      }
      selected = set;
    }
  }

</script>

<!-- 页面根节点由 .content > * 提供布局与激活态 -->
<div class:active={active}>
  <div class="toolbar">
    <div class="button-group">
      <button
        class="icon-button"
        aria-label="批量修改记录"
        title="批量修改记录"
        disabled={selected.size == 0}
        onclick={(e) => {anchor = e.currentTarget; overlayController.open("RecordEdit")}}>
        <PencilIcon size="24"/>
      </button>
      <button
        class="icon-button"
        aria-label="导出记录"
        title="导出记录"
        style="display: none"
        disabled={recordStore.isLoading}
        onclick={() => alert("导出")}>
        <FileArrowDownIcon size="24"/>
      </button>
      <button
        class="icon-button"
        aria-label="刷新"
        title="刷新"
        disabled={recordStore.isLoading}
        onclick={recordStore.load}>
        <ArrowClockwiseIcon size="24"/>
      </button>
    </div>
    <div class="search">
      <MagnifyingGlassIcon size="20"/>
      <input
        type="search"
        disabled={recordStore.isLoading}
        placeholder="搜索姓名、学号或备注"
        bind:value={searchQuery}
      />
    </div>
  </div>

  {#if recordStore.isLoading}
    <div class="page-state">数据加载中...</div>
  {:else if display.length == 0}
    <div class="page-state">暂无历史记录</div>
  {:else}
    <div class="table">
      <table>
        <thead>
        <tr>
          <th class="fixed-width"></th>
          <th><input
            type="checkbox"
            checked={display.length > 0 && displaySelectedCount == display.length}
            indeterminate={displaySelectedCount > 0 && displaySelectedCount < display.length}
            onchange={selectAll}
          /></th>
          <th>序号</th>
          <th>姓名</th>
          <th>学号</th>
          <th>
            <PencilSimpleIcon size="14" weight="bold"/>
            状态
          </th>
          <th>
            <PencilSimpleIcon size="14" weight="bold"/>
            备注
          </th>
          <th>点名时间</th>
        </tr>
        </thead>
        <tbody>
        {#each display as record, index (record.id)}
          {@const color = COLORS[groupInfo[index].groupIndex % COLORS.length]}
          <tr>
            {#if groupInfo[index].isStart}
              <td rowspan={groupInfo[index].rowspan} style:background-color={color} class="fixed-width"></td>
            {/if}
            <td><input
              type="checkbox"
              checked={selected.has(record.id)}
              onchange={() => select(record.id)}
            /></td>
            <td>{index + 1}</td>
            <td>{record.name}</td>
            <td>{record.student_no}</td>
            <td>
              <AttendanceStatusBadge code={record.attendance_status}/>
            </td>
            <td>{record.remark}</td>
            <td>{format(record.rollcall_at)}</td>
          </tr>
        {/each}
        </tbody>
      </table>
    </div>
  {/if}
</div>

<style>
  .page-state {
    flex: 1;
    display: flex;
    align-items: center;
    justify-content: center;
    padding: var(--app-space-xl);
    border-radius: var(--app-radius-sm);
    background: var(--app-color-surface);
    color: var(--app-color-text-muted);
    font-size: var(--app-font-size-sm);
  }

  .fixed-width {
    width: 10px !important;
    padding: 0 !important;
  }
</style>

<EditRecord bind:anchor={anchor} bind:selected={selected}/>
