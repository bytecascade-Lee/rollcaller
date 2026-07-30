<script lang="ts">
  import {studentStore} from "$stores/studentStore.svelte";
  import {format} from "$utils/DataTimeUtils";
  import SingleCreateStudent from "$components/student-management/SingleCreateStudent.svelte";
  import {
    ArrowClockwiseIcon,
    FileArrowDownIcon,
    FileArrowUpIcon,
    MagnifyingGlassIcon,
    MinusIcon,
    PencilIcon,
    PlusIcon
  } from "phosphor-svelte";
  import {studentManagementDialogController} from "$services/studentManagementDialogController";
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

  $effect(() => {
    studentStore.load()
  })
</script>

<div class="page">
  <div class="toolbar">
    <div class="toolbar-button">
      <button onclick={() => studentManagementDialogController.open("SingleCreate")}>
        <PlusIcon/>
        添加
      </button>
      <button onclick={() => studentManagementDialogController.open("Edit")}>
        <PencilIcon/>
        修改
      </button>
      <button onclick={() => (alert("删除"))}>
        <MinusIcon/>
        删除
      </button>
      <button onclick={() => (alert("导入"))}>
        <FileArrowUpIcon/>
        导入
      </button>
      <button onclick={() => (alert("导出"))}>
        <FileArrowDownIcon/>
        导出
      </button>
      <button onclick={() => studentStore.load()}>
        <ArrowClockwiseIcon/>
        刷新
      </button>
    </div>
    <div class="toolbar-search">
      <MagnifyingGlassIcon/>
      <input
        type="search"
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
          <th>姓名</th>
          <th>学号</th>
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
<EditStudent bind:selceted={selected}/>
