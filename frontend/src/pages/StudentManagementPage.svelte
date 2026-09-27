<script lang="ts">
  import {studentStore} from "$stores/studentStore.svelte";
  import {DateTimeUtils} from "$utils";
  import SingleCreateStudent from "$components/student-management/SingleCreateStudent.svelte";
  import DeleteStudents from "$components/student-management/DeleteStudents.svelte";
  import ImportStudents from "$components/student-management/ImportStudents.svelte";
  import {
    ArrowClockwiseIcon,
    ArrowDownIcon,
    ArrowsDownUpIcon,
    ArrowUpIcon,
    ClockClockwiseIcon,
    FileArrowDownIcon,
    FileArrowUpIcon,
    MagnifyingGlassIcon,
    PencilIcon,
    PencilSimpleIcon,
    PlusIcon,
    TrashIcon
  } from "phosphor-svelte";
  import type {EntryRegistry} from "$routing/registry";
  import type {StudentOverlayName} from "$types";
  import {appEnter, readStudentIds} from "$routing/entries";
  import {AppEntry} from "$routing/entries.generated";
  import EditStudent from "$components/student-management/EditStudent.svelte";
  import ExportStudents from "$components/student-management/ExportStudents.svelte";

  let selected = $state<Set<bigint>>(new Set())
  let {active = $bindable(false), overlays}: {
    active?: boolean;
    /** 本页可见的落点表（类型收窄到本页子树：写别的页的名字编译报错） */
    overlays: EntryRegistry<StudentOverlayName>;
  } = $props();
  let searchQuery = $state("")
  let sortKey = $state("")
  let isAsc = $state(true)
  let anchor = $state<HTMLElement | null>(null);
  let display = $derived([...studentStore.students]
    .filter(student =>
      student.name.toLowerCase().includes(searchQuery) ||
      student.student_no.toLowerCase().includes(searchQuery)
    )
    .sort((a, b) => {
      if (!sortKey) return 0;
      const key = sortKey as "name" | "student_no" | "created_at" | "updated_at";
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

  /**
   * 生效的选择集：按"学生仍然存在"过滤。
   *
   * 于是删除后自动消失 —— 不必靠"记得调用 clear()"来消除悬空 id；
   * "载荷里的 id 已不存在"也随之退化为普通情况，而不是特例。
   */
  let liveSelected = $derived.by(() => {
    const existing = new Set(studentStore.students.map((student) => student.id));
    return new Set([...selected].filter((id) => existing.has(id)));
  });

  /** 只用于渲染勾选态与表头全选 */
  let displaySelectedCount = $derived(display.filter(student => liveSelected.has(student.id)).length)

  function sort(key: string) {
    if (sortKey === key) {
      isAsc = !isAsc;
    } else {
      sortKey = key;
      isAsc = true;
    }
  }

  function select(id: bigint) {
    if (selected.has(id)) {
      let set = new Set(selected);
      set.delete(id)
      selected = set;
    } else {
      selected = new Set([...selected, id]);
    }
  }

  /**
   * 表头全选：只作用于**当前可见**的行，且不丢掉看不见的选中项。
   *
   * 用 `displaySelectedCount` 判断"是否已全选可见行"，故与表头勾选态永远一致 ——
   * 旧实现作用于全体学生，带筛选时表头会撒谎。
   */
  function selectAll() {
    if (display.length > 0 && displaySelectedCount === display.length) {
      const rest = new Set(selected);
      for (const student of display) rest.delete(student.id);
      selected = rest;
    } else {
      selected = new Set([...selected, ...display.map((student) => student.id)]);
    }
  }

  /**
   * 载荷解释器：本页是 `{ids}` 的**唯一解释者**。
   *
   * 路由层不认识 `{ids}`；弹窗也不必认识 —— 它只管可见性，由注册表在 `show` 之前调用这里，
   * 把"编辑这些学生"翻译成本页状态（选中集）。载荷非法或为空 ⇒ 什么都不做（纯导航）。
   *
   * 前置条件（"要看到选中效果，学生数据得在"）由 `liveSelected` 的过滤自然覆盖：
   * 先写入 id，数据到达后过滤结果自动生效，故这里不必等 `load()`。
   */
  $effect(() => {
    overlays.interpret(AppEntry.editStudent, (payload) => {
      const ids = readStudentIds(payload);
      if (ids.length === 0) return;
      selected = new Set(ids.map((id) => BigInt(id)));
      if (studentStore.students.length === 0) void studentStore.load();
    });
  });

</script>

<!-- 页面根节点由 .content > * 提供布局与激活态 -->
<div class:active={active}>
  <div class="toolbar">
    <div class="icon-button-group">
      <button
        aria-label="添加学生"
        class="icon-button"
        disabled={studentStore.isLoading}
        onclick={() => appEnter(AppEntry.createStudent)}
        title="添加学生"
      >
        <PlusIcon size="24"/>
      </button>
      <button
        aria-label="修改学生"
        class="icon-button"
        disabled={liveSelected.size != 1}
        onclick={() => appEnter(AppEntry.editStudent, {ids: [...liveSelected].map(Number)})}
        title="修改学生"
      >
        <PencilIcon size="24"/>
      </button>
      <button
        aria-label="删除学生"
        class="icon-button"
        disabled={liveSelected.size == 0}
        onclick={() => appEnter(AppEntry.deleteStudent, {ids: [...liveSelected].map(Number)})}
        title="删除学生"
      >
        <TrashIcon size="24"/>
      </button>
      <button
        aria-label="导入学生"
        class="icon-button"
        disabled={studentStore.isLoading}
        onclick={() => appEnter(AppEntry.importStudent)}
        title="导入学生"
      >
        <FileArrowUpIcon size="24"/>
      </button>
      <button
        aria-label="导出学生"
        class="icon-button"
        disabled={studentStore.isLoading}
        onclick={e => {
          anchor = e.currentTarget;
          appEnter(AppEntry.exportStudent)
        }}
        title="导出学生"
      >
        <FileArrowDownIcon size="24"/>
      </button>
      <!-- 功能后期添加 -->
      <button
        aria-label="恢复已删除学生"
        class="icon-button"
        onclick={() => console.warn("[TODO] restore 尚未实现：恢复已删除学生的入口还没接上")}
        style="display: none"
        title="恢复已删除学生"
      >
        <ClockClockwiseIcon size="24"/>
      </button>
      <button
        aria-label="刷新"
        class="icon-button"
        disabled={studentStore.isLoading}
        onclick={() => studentStore.load()}
        title="刷新"
      >
        <ArrowClockwiseIcon size="24"/>
      </button>
    </div>
    <div class="search">
      <MagnifyingGlassIcon size="18"/>
      <input
        bind:value={searchQuery}
        disabled={studentStore.isLoading}
        placeholder="搜索学号或姓名"
        type="search"
      />
    </div>
  </div>

  {#if studentStore.isLoading}
    <div class="state">数据加载中...</div>
  {:else if display.length == 0}
    <div class="state">暂无学生数据</div>
  {:else}
    <div class="table">
      <table>
        <thead>
        <tr>
          <th>
            <input
              type="checkbox"
              checked={display.length > 0 && displaySelectedCount == display.length}
              indeterminate={displaySelectedCount > 0 && displaySelectedCount < display.length}
              onchange={selectAll}/>
          </th>
          <th style:cursor="auto">序号</th>
          <th onclick={() => sort("name")}>
            <PencilSimpleIcon size="14" weight="bold"/>
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
            <PencilSimpleIcon size="14" weight="bold"/>
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
          <th onclick={() => sort("created_at")}>
            创建时间
            {#if sortKey === "created_at"}
              {#if isAsc}
                <ArrowUpIcon size="14" weight="bold" color="var(--color-primary)"/>
              {:else}
                <ArrowDownIcon size="14" weight="bold" color="var(--color-primary)"/>
              {/if}
            {:else}
              <ArrowsDownUpIcon size="14"/>
            {/if}
          </th>
          <th onclick={() => sort("updated_at")}>
            最后更新时间
            {#if sortKey === "updated_at"}
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
        {#each display as student, index (student.id)}
          <tr>
            <td>
              <input
                type="checkbox"
                checked={selected.has(student.id)}
                onchange={() => select(student.id)}
              />
            </td>
            <td>{index + 1}</td>
            <td>{student.name}</td>
            <td>{student.student_no}</td>
            <td>{DateTimeUtils.format(student.created_at)}</td>
            <td>{DateTimeUtils.format(student.updated_at)}</td>
          </tr>
        {/each}
        </tbody>
      </table>
    </div>
  {/if}
</div>

<SingleCreateStudent {overlays}/>
<EditStudent {overlays} selected={liveSelected}/>
<DeleteStudents {overlays}/>
<ImportStudents {overlays}/>
<ExportStudents anchor={anchor} display={display} {overlays} selected={liveSelected}/>
