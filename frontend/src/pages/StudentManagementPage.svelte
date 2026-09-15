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
  import {createOverlayRegistry} from "$routing/overlays";
  import type {StudentOverlayName} from "$routing/overlayNames";
  import {parseRoute, type Action, type Payload} from "$routing/route";
  import {untrack} from "svelte";
  import EditStudent from "$components/student-management/EditStudent.svelte";
  import ExportStudents from "$components/student-management/ExportStudents.svelte";

  /** 本页的叠加层注册表：注册表归页面所有，弹窗逐个注册进来（不再是全局单例） */
  const overlays = createOverlayRegistry<StudentOverlayName>();

  let selected = $state<Set<bigint>>(new Set())
  let {active = $bindable(false), action = null}: {
    active?: boolean;
    /** 路由投递过来的完整 Action：把载荷翻译成本页状态是本页的私事 */
    action?: Action | null;
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
  let displaySelectedCount = $derived(display.filter(student => selected.has(student.id)).length)

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

  function selectAll() {
    if (selected.size == studentStore.students.length) {
      selected = new Set<bigint>();
    } else {
      let set = new Set<bigint>();
      for (let student of studentStore.students) {
        set.add(student.id);
      }
      selected = set;
    }
  }

  /** 载荷校验：`{ids: number[]}`。不合法一律返回空数组，让调用方降级为纯导航 */
  function idsOf(payload: Payload | undefined): number[] {
    const raw = payload?.ids;
    if (!Array.isArray(raw)) return [];
    return raw.filter((id): id is number => Number.isSafeInteger(id));
  }

  /**
   * 地址落地：本页是**载荷的唯一解释者**。
   *
   * 路由层不认识 `{ids}` 是什么，只有这里知道"选中的是谁"存在本页的 `selected`（`Set<bigint>`）
   * 里、编辑界面在注册表中叫 `"edit"`——发起方不需要也不该知道这些。
   *
   * 降级：段不匹配或载荷非法/为空 → 什么都不做（地址已经把卡片切过来了，纯导航）。
   */
  async function apply(act: Action) {
    const segments = parseRoute(act.route)?.segments ?? [];
    if (segments[1] !== "edit") return;
    const ids = idsOf(act.payload);
    if (ids.length === 0) return;
    // 异步前置：已挂载 ≠ 可应用，数据没到就选中会落空
    if (studentStore.students.length === 0) await studentStore.load();
    selected = new Set(ids.map((id) => BigInt(id)));
    overlays.open("edit");
  }

  // 用 untrack 把 apply 读到的 store 挡在依赖之外：载荷的应用只应由 action 变化触发，
  // 否则数据加载完会再触发一次（重复打开、重复重置表单）。
  $effect(() => {
    const act = action;
    if (!act) return;
    untrack(() => void apply(act));
  });

</script>

<!-- 页面根节点由 .content > * 提供布局与激活态 -->
<div class:active={active}>
  <div class="toolbar">
    <div class="icon-button-group">
      <button
        class="icon-button"
        aria-label="添加学生"
        title="添加学生"
        disabled={studentStore.isLoading}
        onclick={() => overlays.open("create")}
      >
        <PlusIcon size="24"/>
      </button>
      <button
        class="icon-button"
        aria-label="修改学生"
        title="修改学生"
        disabled={selected.size != 1}
        onclick={() => overlays.open("edit")}
      >
        <PencilIcon size="24"/>
      </button>
      <button
        class="icon-button"
        aria-label="删除学生"
        title="删除学生"
        disabled={selected.size == 0}
        onclick={() => overlays.open("delete")}
      >
        <TrashIcon size="24"/>
      </button>
      <button
        class="icon-button"
        aria-label="导入学生"
        title="导入学生"
        disabled={studentStore.isLoading}
        onclick={() => overlays.open("import")}
      >
        <FileArrowUpIcon size="24"/>
      </button>
      <button
        class="icon-button"
        aria-label="导出学生"
        title="导出学生"
        disabled={studentStore.isLoading}
        onclick={e => {
          anchor = e.currentTarget;
          overlays.open("export")
        }}
      >
        <FileArrowDownIcon size="24"/>
      </button>
      <!-- 功能后期添加 -->
      <button
        class="icon-button"
        aria-label="恢复已删除学生"
        title="恢复已删除学生"
        style="display: none"
        onclick={() => overlays.open("restore")}
      >
        <ClockClockwiseIcon size="24"/>
      </button>
      <button
        class="icon-button"
        aria-label="刷新"
        title="刷新"
        disabled={studentStore.isLoading}
        onclick={() => studentStore.load()}
      >
        <ArrowClockwiseIcon size="24"/>
      </button>
    </div>
    <div class="search">
      <MagnifyingGlassIcon size="18"/>
      <input
        type="search"
        disabled={studentStore.isLoading}
        placeholder="搜索学号或姓名"
        bind:value={searchQuery}
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
              onchange={selectAll}
            />
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
<EditStudent bind:selected={selected} {overlays}/>
<DeleteStudents bind:selected={selected} {overlays}/>
<ImportStudents {overlays}/>
<ExportStudents bind:selected={selected} bind:display={display} bind:anchor={anchor} {overlays}/>
