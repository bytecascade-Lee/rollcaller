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
  import {studentManagementDialogController} from "$controllers/studentManagementDialogController";
  import EditStudent from "$components/student-management/EditStudent.svelte";

  let selected = $state<Set<bigint>>(new Set())
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

<div class="page">
  <div class="toolbar">
    <div class="toolbar-button">
      <button
        disabled={studentStore.isLoading}
        onclick={() => studentManagementDialogController.open("SingleCreate")}>
        <PlusIcon/>
        添加
      </button>
      <button
        disabled={selected.size != 1}
        onclick={() => studentManagementDialogController.open("Edit")}>
        <PencilIcon/>
        修改
      </button>
      <button
        disabled={selected.size == 0}
        onclick={() => studentManagementDialogController.open("Delete")}>
        <MinusIcon/>
        删除
      </button>
      <button
        disabled={studentStore.isLoading}
        onclick={() => studentManagementDialogController.open("Import")}>
        <FileArrowUpIcon/>
        导入
      </button>
      <button
        disabled={studentStore.isLoading}
        onclick={() => (alert("导出"))}>
        <FileArrowDownIcon/>
        导出
      </button>
      <button
        disabled={studentStore.isLoading}
        onclick={() => studentStore.load()}>
        <ArrowClockwiseIcon/>
        刷新
      </button>
    </div>
    <div class="toolbar-search">
      <MagnifyingGlassIcon/>
      <input
        type="search"
        disabled={studentStore.isLoading}
        placeholder="🔍 搜索学号或姓名"
        bind:value={searchQuery}/>
    </div>
  </div>

  {#if studentStore.isLoading}
    数据加载中...
  {:else if display.length == 0}
    暂无学生数据
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
            <PencilSimpleIcon/>
            姓名
          </th>
          <th>
            <PencilSimpleIcon/>
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

<SingleCreateStudent/>
<EditStudent bind:selected={selected}/>
<DeleteStudents bind:selected={selected}/>
<ImportStudents/>
