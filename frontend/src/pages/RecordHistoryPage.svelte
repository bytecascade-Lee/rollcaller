<script lang="ts">

  import {recordStore} from "$stores/recordStore.svelte";
  import {format} from "$utils/DataTimeUtils";
  import type {RecordGroupMetaData, RollcallRecord} from "$types";
  import {
    ArrowClockwiseIcon,
    ArrowDownIcon,
    ArrowsDownUpIcon,
    ArrowUpIcon,
    FileArrowDownIcon,
    MagnifyingGlassIcon,
    PencilIcon,
    PencilSimpleIcon
  } from "phosphor-svelte"
  import {COLORS, group} from "$services/RecordService.svelte";
  import AttendanceStatusBadge from "$components/record-history/AttendanceStatusBadge.svelte";
  import EditRecord from "$components/record-history/EditRecord.svelte";
  import {overlayController} from "$controllers/popupController";
  import ExportRecords from "$components/record-history/ExportRecords.svelte";

  let selected = $state<Set<bigint>>(new Set());
  let {active = $bindable(false)} = $props();
  let sortKey = $state("")
  let isAsc = $state(true)
  let searchQuery = $state("");
  let anchor = $state<HTMLElement | null>(null);

  let display = $derived<RollcallRecord[]>([...recordStore.records]
    .filter((record) => {
      return (
        record.student_no.toLowerCase().includes(searchQuery) ||
        record.name.toLowerCase().includes(searchQuery) ||
        record.remark?.toLowerCase().includes(searchQuery) ||
        format(record.rollcall_at).toLowerCase().includes(searchQuery)
      );
    })
    .sort((a, b) => {
      if (!sortKey) return 0;
      const key = sortKey as "name" | "student_no" | "attendance_status" | "remark" | "rollcall_at";
      const valA = a[key];
      const valB = b[key];
      let cmp: number;
      if (typeof valA === "string" && typeof valB === "string") {
        cmp = valA.localeCompare(valB, "zh-Hans-CN");
      } else if (typeof valA === "number" && typeof valB === "number") {
        cmp = valA - valB;
      } else {
        cmp = 0;
      }
      return isAsc ? cmp : -cmp;
    }));

  function sort(key: string) {
    if (sortKey === key) {
      isAsc = !isAsc;
    } else {
      sortKey = key;
      isAsc = true;
    }
  }

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
    <div class="icon-button-group">
      <button
        class="icon-button"
        aria-label="批量修改记录"
        title="批量修改记录"
        disabled={selected.size == 0}
        onclick={e => {
          anchor = e.currentTarget;
          overlayController.open("RecordEdit")
        }}
      >
        <PencilIcon size="24"/>
      </button>
      <button
        class="icon-button"
        aria-label="导出记录"
        title="导出记录"
        disabled={recordStore.isLoading}
        onclick={e => {
          anchor = e.currentTarget;
          overlayController.open("RecordExport")
        }}
      >
        <FileArrowDownIcon size="24"/>
      </button>
      <button
        class="icon-button"
        aria-label="刷新"
        title="刷新"
        disabled={recordStore.isLoading}
        onclick={() => recordStore.load()}
      >
        <ArrowClockwiseIcon size="24"/>
      </button>
    </div>
    <div class="search">
      <MagnifyingGlassIcon size="18"/>
      <input
        type="search"
        disabled={recordStore.isLoading}
        placeholder="搜索学号、姓名或备注"
        bind:value={searchQuery}
      />
    </div>
  </div>

  {#if recordStore.isLoading}
    <div class="state">数据加载中...</div>
  {:else if display.length == 0}
    <div class="state">暂无历史记录</div>
  {:else}
    <div class="table">
      <table>
        <thead>
        <tr>
          <th
            style:border="0px"
            style:width="8px"
          ></th>
          <th><input
            type="checkbox"
            checked={display.length > 0 && displaySelectedCount == display.length}
            indeterminate={displaySelectedCount > 0 && displaySelectedCount < display.length}
            onchange={selectAll}
          /></th>
          <th style:cursor="auto">序号</th>
          <th onclick={() => sort("name")}>
            姓名
            {#if sortKey === "name"}
              {#if isAsc}
                <ArrowUpIcon size="14" weight="bold" color="var(--color-primary)"/>
              {:else}
                <ArrowDownIcon size="14" weight="bold" color="var(--color-primary)"/>
              {/if}
            {:else}
              <ArrowsDownUpIcon size="14"/>{/if}
          </th>
          <th onclick={() => sort("student_no")}>
            学号
            {#if sortKey === "student_no"}
              {#if isAsc}
                <ArrowUpIcon size="14" weight="bold" color="var(--color-primary)"/>
              {:else}
                <ArrowDownIcon size="14" weight="bold" color="var(--color-primary)"/>
              {/if}
            {:else}
              <ArrowsDownUpIcon size="14"/>{/if}
          </th>
          <th onclick={() => sort("attendance_status")}>
            <PencilSimpleIcon size="14" weight="bold"/>
            状态
            {#if sortKey === "attendance_status"}
              {#if isAsc}
                <ArrowUpIcon size="14" weight="bold" color="var(--color-primary)"/>
              {:else}
                <ArrowDownIcon size="14" weight="bold" color="var(--color-primary)"/>
              {/if}
            {:else}
              <ArrowsDownUpIcon size="14"/>{/if}
          </th>
          <th onclick={() => sort("remark")}>
            <PencilSimpleIcon size="14" weight="bold"/>
            备注
            {#if sortKey === "remark"}
              {#if isAsc}
                <ArrowUpIcon size="14" weight="bold" color="var(--color-primary)"/>
              {:else}
                <ArrowDownIcon size="14" weight="bold" color="var(--color-primary)"/>
              {/if}
            {:else}
              <ArrowsDownUpIcon size="14"/>
            {/if}
          </th>
          <th onclick={() => sort("rollcall_at")}>
            点名时间
            {#if sortKey === "rollcall_at"}
              {#if isAsc}
                <ArrowUpIcon size="14" weight="bold" color="var(--color-primary)"/>
              {:else}
                <ArrowDownIcon size="14" weight="bold" color="var(--color-primary)"/>
              {/if}
            {:else}
              <ArrowsDownUpIcon size="14"/>
            {/if}
          </th>
        </tr>
        </thead>
        <tbody>
        {#each display as record, index (record.id)}
          {@const color = COLORS[groupInfo[index].groupIndex % COLORS.length]}
          <tr>
            {#if groupInfo[index].isStart}
              <td
                rowspan={groupInfo[index].rowspan}
                style:background={color}
                style:border-bottom="0px"
              ></td>
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
            <td
              style:white-space="normal"
              style:word-wrap="break-word"
            >
              {record.remark}
            </td>
            <td>{format(record.rollcall_at)}</td>
          </tr>
        {/each}
        </tbody>
      </table>
    </div>
  {/if}
</div>

<EditRecord bind:anchor={anchor} bind:selected={selected}/>
<ExportRecords bind:selected={selected} bind:display={display} bind:anchor={anchor}/>
