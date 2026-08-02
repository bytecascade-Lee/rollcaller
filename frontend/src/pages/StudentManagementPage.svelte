<script lang="ts">
  import {studentStore} from "$stores/studentStore.svelte";
  import {format} from "$utils/DataTimeUtils";
  import SingleCreateStudent from "$components/student-management/SingleCreateStudent.svelte";
  import DeleteStudents from "$components/student-management/DeleteStudents.svelte";
  import ImportStudents from "$components/student-management/ImportStudents.svelte";
  import {
    ArrowClockwiseIcon,
    FileArrowDownIcon,
    FileArrowUpIcon,
    MagnifyingGlassIcon,
    MinusIcon,
    PencilIcon,
    PencilSimpleIcon,
    PlusIcon
  } from "phosphor-svelte";
  import {overlayController} from "$controllers/overlayController";
  import EditStudent from "$components/student-management/EditStudent.svelte";
  import ExportStudents from "$components/student-management/ExportStudents.svelte";

  let selected = $state<Set<bigint>>(new Set())
  let {active = $bindable(false)} = $props();
  let searchQuery = $state("")
  let display = $derived(
    studentStore.students.filter(student =>
      student.name.toLowerCase().includes(searchQuery) ||
      student.student_no.toLowerCase().includes(searchQuery)
    ));
  let displaySelectedCount = $derived(display.filter(student => selected.has(student.id)).length)

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
    <div class="button-group">
      <button
        class="icon-button"
        aria-label="添加学生"
        title="添加学生"
        disabled={studentStore.isLoading}
        onclick={() => overlayController.open("StudentSingleCreate")}>
        <PlusIcon size="24"/>
      </button>
      <button
        class="icon-button"
        aria-label="修改学生"
        title="修改学生"
        disabled={selected.size != 1}
        onclick={() => overlayController.open("StudentEdit")}>
        <PencilIcon size="24"/>
      </button>
      <button
        class="icon-button"
        aria-label="删除学生"
        title="删除学生"
        disabled={selected.size == 0}
        onclick={() => overlayController.open("StudentDelete")}>
        <MinusIcon size="24"/>
      </button>
      <button
        class="icon-button"
        aria-label="导入学生"
        title="导入学生"
        disabled={studentStore.isLoading}
        onclick={() => overlayController.open("StudentImport")}>
        <FileArrowUpIcon size="24"/>
      </button>
      <button
        class="icon-button"
        aria-label="导出学生"
        title="导出学生"
        disabled={studentStore.isLoading}
        onclick={() => overlayController.open("StudentExport")}>
        <FileArrowDownIcon size="24"/>
      </button>
      <button
        class="icon-button"
        aria-label="刷新"
        title="刷新"
        disabled={studentStore.isLoading}
        onclick={() => studentStore.load()}>
        <ArrowClockwiseIcon size="24"/>
      </button>
    </div>
    <div class="search">
      <MagnifyingGlassIcon size="24"/>
      <input
        type="search"
        disabled={studentStore.isLoading}
        placeholder="搜索学号或姓名"
        bind:value={searchQuery}/>
    </div>
  </div>

  {#if studentStore.isLoading}
    <div class="page-state">数据加载中...</div>
  {:else if display.length == 0}
    <div class="page-state">暂无学生数据</div>
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
          <th>序号</th>
          <th>
            <PencilSimpleIcon size="14" weight="bold"/>
            姓名
          </th>
          <th>
            <PencilSimpleIcon size="14" weight="bold"/>
            学号
          </th>
          <th>创建时间</th>
          <th>最后更新时间</th>
        </tr>
        </thead>
        <tbody>
        {#each display as student, index (student.id)}
          <tr>
            <td>
              <input
                type="checkbox"
                checked={selected.has(student.id)}
                onchange={() => select(student.id)}/>
            </td>
            <td>{index + 1}</td>
            <td>{student.name}</td>
            <td>{student.student_no}</td>
            <td>{format(student.created_at)}</td>
            <td>{format(student.updated_at)}</td>
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
    background: var(--app-color-page);
    color: var(--app-color-text);
    font-size: var(--app-font-size-bg);
  }
</style>

<SingleCreateStudent/>
<EditStudent bind:selected={selected}/>
<DeleteStudents bind:selected={selected}/>
<ImportStudents/>
<ExportStudents bind:selected={selected}/>
