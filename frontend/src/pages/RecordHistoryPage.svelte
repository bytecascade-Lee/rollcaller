<script lang="ts">

  import {recordStore} from "$stores/recordStore.svelte";
  import {DateTimeUtils} from "$utils";
  import type {RecordGroupMetaData, RecordOverlayName, RollcallRecord} from "$types";
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
  import {recordService} from "$services/RecordService.svelte";
  import AttendanceStatusBadge from "$components/record-history/AttendanceStatusBadge.svelte";
  import EditRecord from "$components/record-history/EditRecord.svelte";
  import type {EntryRegistry} from "$routing/registry";
  import {AppEntry} from "$routing/entries.generated";
  import ExportRecords from "$components/record-history/ExportRecords.svelte";

  let selected = $state<Set<bigint>>(new Set());
  let {active = $bindable(false), overlays}: {
    active?: boolean;
    /** 本页可见的落点表（类型收窄到本页子树） */
    overlays: EntryRegistry<RecordOverlayName>;
  } = $props();
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
        DateTimeUtils.format(record.rollcall_at).toLowerCase().includes(searchQuery)
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
      // 如果是按照点名时间排序，则先倒序
      // 其他的则是正常情况，正序
      isAsc = sortKey != "rollcall_at";
    }
  }

  let groupInfo = $derived<RecordGroupMetaData[]>(recordService.group(display));

  /** 生效的选择集：按"记录仍然存在"过滤（与学生管理页同一套语义） */
  let liveSelected = $derived.by(() => {
    const existing = new Set(recordStore.records.map((record) => record.id));
    return new Set([...selected].filter((id) => existing.has(id)));
  });

  let displaySelectedCount = $derived(display.filter(r => liveSelected.has(r.id)).length)

  export function select(id: bigint) {
    if (selected.has(id)) {
      let set = new Set(selected);
      set.delete(id)
      selected = set;
    } else {
      selected = new Set([...selected, id]);
    }
  }

  /** 子组件回报新的选择集：选中集归本页所有，弹窗不直接改它 */
  export function setSelection(ids: readonly bigint[]) {
    selected = new Set(ids);
  }

  /** 表头全选：只作用于当前可见的行，且不丢掉看不见的选中项 */
  export function selectAll() {
    if (display.length > 0 && displaySelectedCount === display.length) {
      const rest = new Set(selected);
      for (const record of display) rest.delete(record.id);
      selected = rest;
    } else {
      selected = new Set([...selected, ...display.map((record) => record.id)]);
    }
  }

</script>

<!-- 页面根节点由 .content > * 提供布局与激活态 -->
<div class:active={active}>
  <div class="toolbar">
    <div class="icon-button-group">
      <button
        aria-label="批量修改记录"
        class="icon-button"
        disabled={liveSelected.size == 0}
        onclick={e => {
          anchor = e.currentTarget;
          overlays.show(AppEntry.editRecord)
        }}
        title="批量修改记录"
      >
        <PencilIcon size="24"/>
      </button>
      <button
        aria-label="导出记录"
        class="icon-button"
        disabled={recordStore.isLoading}
        onclick={e => {
          anchor = e.currentTarget;
          overlays.show(AppEntry.exportRecord)
        }}
        title="导出记录"
      >
        <FileArrowDownIcon size="24"/>
      </button>
      <button
        aria-label="刷新"
        class="icon-button"
        disabled={recordStore.isLoading}
        onclick={() => recordStore.load()}
        title="刷新"
      >
        <ArrowClockwiseIcon size="24"/>
      </button>
    </div>
    <div class="search">
      <MagnifyingGlassIcon size="18"/>
      <input
        bind:value={searchQuery}
        disabled={recordStore.isLoading}
        placeholder="搜索学号、姓名或备注"
        type="search"
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
              <ArrowsDownUpIcon size="14"/>
            {/if}
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
              <ArrowsDownUpIcon size="14"/>
            {/if}
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
              <ArrowsDownUpIcon size="14"/>
            {/if}
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
          {@const color = recordService.groupColors[groupInfo[index].groupIndex % recordService.groupColors.length]}
          <tr>
            {#if groupInfo[index].isStart}
              <!-- 当按照点名时间排序，或按照默认排序（等于空字符串时），显示分组信息 -->
              <!-- 否则隐藏分组信息 -->
              <!-- 至于使用 visibility 而非 display='none'，是防止表结构来回变动，干扰视觉 -->
              <td
                rowspan={groupInfo[index].rowspan}
                style:background={color}
                style:border-bottom="0px"
                style:visibility={(sortKey == "rollcall_at" || sortKey == "") ? "visible" : "hidden"}
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
            <td onclick={() => recordService.updateToNextStatus(record.id, record.attendance_status)}>
              <AttendanceStatusBadge id={record.attendance_status}/>
            </td>
            <td
              style:white-space="normal"
              style:word-wrap="break-word"
            >
              {record.remark}
            </td>
            <td>{DateTimeUtils.format(record.rollcall_at)}</td>
          </tr>
        {/each}
        </tbody>
      </table>
    </div>
  {/if}
</div>

<EditRecord anchor={anchor} onselect={setSelection} {overlays} selected={liveSelected}/>
<ExportRecords anchor={anchor} display={display} {overlays} selected={liveSelected}/>
