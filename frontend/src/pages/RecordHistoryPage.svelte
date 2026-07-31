<script lang="ts">

  import {recordStore} from "$stores/recordStore.svelte";
  import {format} from "$utils/DataTimeUtils";
  import type {RollcallRecord} from "$types/RollcallRecord";
  import {statusText} from "$constants/AttendanceStatus";
  import type {RecordGroupMetaData} from "$types/RecordGroupMetaData"
  import {ArrowClockwiseIcon, FileArrowDownIcon, MagnifyingGlassIcon, PencilIcon, PencilSimpleIcon} from "phosphor-svelte"
  import {COLORS, group} from "$services/RecordService.svelte";

  let selected = $state<Set<bigint>>(new Set());
  let searchQuery = $state("");

  let display = $derived.by<RollcallRecord[]>(() => {
    if (!searchQuery.trim()) return recordStore.records;
    const q = searchQuery.trim().toLowerCase();
    return recordStore.records.filter((r) => {
      return (
        r.student_no.toLowerCase().includes(q) ||
        r.name.toLowerCase().includes(q) ||
        r.remark.toLowerCase().includes(q) ||
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

<div class="page">
  <div class="toolbar">
    <div class="toolbar-button">
      <button
        disabled={selected.size == 0}>
        <PencilIcon/>
        修改
      </button>
      <button
        disabled={recordStore.isLoading}
        onclick={() => alert("导出")}>
        <FileArrowDownIcon/>
        导出
      </button>
      <button
        disabled={recordStore.isLoading}
        onclick={recordStore.load}>
        <ArrowClockwiseIcon/>
        刷新
      </button>
    </div>
    <div class="toolbar-search">
      <MagnifyingGlassIcon/>
      <input
        type="search"
        disabled={recordStore.isLoading}
        placeholder="搜索姓名、学号或备注"
        bind:value={searchQuery}
      />
    </div>
  </div>

  {#if recordStore.isLoading}
    数据加载中...
  {:else if display.length == 0}
    暂无历史记录
  {:else}
    <div class="table">
      <table>
        <thead>
        <tr>
          <th></th>
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
            <PencilSimpleIcon/>
            状态
          </th>
          <th>
            <PencilSimpleIcon/>
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
              <td rowspan={groupInfo[index].rowspan} style:background-color={color}></td>
            {/if}
            <td><input
              type="checkbox"
              checked={selected.has(record.id)}
              onchange={() => select(record.id)}
            /></td>
            <td>{index + 1}</td>
            <td>{record.name}</td>
            <td>{record.student_no}</td>
            <td>{statusText(record.attendance_status)}</td>
            <td>{record.remark}</td>
            <td>{format(record.rollcall_at)}</td>
          </tr>
        {/each}
        </tbody>
      </table>
    </div>
  {/if}
</div>
