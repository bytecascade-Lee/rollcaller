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
  import {overlayController} from "$controllers/popupController";
  import EditStudent from "$components/student-management/EditStudent.svelte";
  import ExportStudents from "$components/student-management/ExportStudents.svelte";

  let selected = $state<Set<bigint>>(new Set())
  let {active = $bindable(false)} = $props();
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
        onclick={() => overlayController.open("StudentSingleCreate")}
      >
        <PlusIcon size="24"/>
      </button>
      <button
        class="icon-button"
        aria-label="修改学生"
        title="修改学生"
        disabled={selected.size != 1}
        onclick={() => overlayController.open("StudentEdit")}
      >
        <PencilIcon size="24"/>
      </button>
      <button
        class="icon-button"
        aria-label="删除学生"
        title="删除学生"
        disabled={selected.size == 0}
        onclick={() => overlayController.open("StudentDelete")}
      >
        <TrashIcon size="24"/>
      </button>
      <button
        class="icon-button"
        aria-label="导入学生"
        title="导入学生"
        disabled={studentStore.isLoading}
        onclick={() => overlayController.open("StudentImport")}
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
          overlayController.open("StudentExport")
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
        onclick={() => overlayController.open("StudentRestore")}
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

<SingleCreateStudent/>
<EditStudent bind:selected={selected}/>
<DeleteStudents bind:selected={selected}/>
<ImportStudents/>
<ExportStudents bind:selected={selected} bind:display={display} bind:anchor={anchor}/>
