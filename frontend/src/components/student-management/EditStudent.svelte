<script lang="ts">
  import {studentStore} from "$stores/studentStore.svelte";
  import {invoke} from "@tauri-apps/api/core";
  import {studentManagementDialogController} from "$controllers/studentManagementDialogController";
  import type {StudentTable} from "$types/StudentTable";


  let {selected = $bindable()}: { selected: Set<bigint> } = $props();
  let value = $derived(selected.values().next().value);
  let editing = $derived(studentStore.get(value ? value : -1n))

  let isVisible = $state(false);
  let closeOnOutside = true;

  async function edit() {
    if (!editing || !editing.name.trim() || !editing.student_no.trim()) return;
    try {
      let student = await invoke<StudentTable>("student_single_update", {
        student: {
          id: editing.id,
          student_no: editing.student_no.trim(),
          name: editing.name.trim(),
        }
      });
      studentStore.upsert(student);
    } catch (e) {
      alert(String(e));
    }
  }

  $effect(() => {
    studentManagementDialogController.register("Edit", {
      open: () => isVisible = true,
      close: () => isVisible = false,
      isVisible: () => isVisible
    })
  })
</script>

{#if isVisible}
  <!-- svelte-ignore a11y_click_events_have_key_events -->
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div class="overlay" onclick={closeOnOutside ? () => isVisible = false : undefined}>
    <div class="dialog" onclick={(e) => e.stopPropagation()}>
      {#if selected.size > 1}
        <h3>仅支持单个修改</h3>
        <div>
          <button onclick={() => isVisible = false}>确定</button>
        </div>
      {:else if editing == null}
        <h3>待编辑的对象为Null，失败！</h3>
        <div>当前选中id：{Array.from(selected)}</div>
        <div>当前查找的id：{value ? value : "undefined"}</div>
        <div>当前获取到的编辑对象：{editing}</div>
        <div>
          <button onclick={() => isVisible = false}>确定</button>
        </div>
      {:else}
        <!-- 此处无需bind，否则会直接修改表格数据，应该等到写入库中后再修改 -->
        <h3>修改学生</h3>
        <label>
          学号
          <input type="text" value={editing.student_no}/>
        </label>
        <label>
          姓名
          <input type="text" value={editing.name}/>
        </label>
        <div class="dialog-actions">
          <button class="btn-secondary" onclick={() => isVisible = false}>取消</button>
          <button onclick={edit}>保存</button>
        </div>
      {/if}
    </div>
  </div>
{/if}
