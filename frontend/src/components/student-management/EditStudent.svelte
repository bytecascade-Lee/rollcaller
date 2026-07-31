<script lang="ts">
  import {studentStore} from "$stores/studentStore.svelte";
  import {studentManagementDialogController} from "$controllers/studentManagementDialogController";
  import type {StudentTable} from "$types/StudentTable";
  import type {StudentSingleUpdate} from "$types/StudentSingleUpdate";
  import {StudentCommand} from "$commands"
  import {format} from "$utils/DataTimeUtils";

  let {selected = $bindable()}: { selected: Set<bigint> } = $props();
  let localEdit = $state<StudentTable | null>(null);
  let isVisible = $state(false);
  let closeOnOutside = true;
  let editResult = $state<StudentSingleUpdate>()

  async function edit() {
    if (!localEdit) return;
    localEdit.student_no = localEdit.student_no.trim()
    localEdit.name = localEdit.name.trim()
    if (!localEdit.name || !localEdit.student_no) return;
    try {
      editResult = await StudentCommand.update({
          id: localEdit.id,
          student_no: localEdit.student_no,
          name: localEdit.name,
        }
      );
      if (editResult && editResult.type == "Update") {
        studentStore.upsert(localEdit);
        close()
      }
    } catch (e) {
      alert(String(e));
    }
  }

  function close() {
    isVisible = false;
    localEdit = null;
    editResult = undefined;
  }

  $effect(() => {
    if (isVisible && selected.size == 1) {
      // 必须拷贝，否则拿到的是引用，表格中的照样会变
      const original = studentStore.get(selected.values().next().value);
      localEdit = original ? {...original} : null;
    }
  });
  $effect(() => {
    studentManagementDialogController.register("Edit", {
      open: () => isVisible = true,
      close: close,
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
      {:else if localEdit == null}
        <h3>待编辑的对象为Null，失败！</h3>
        <div>当前选中id：{Array.from(selected)}</div>
        <div>
          <button onclick={() => isVisible = false}>确定</button>
        </div>
      {:else}
        <h3>修改学生</h3>
        <label>
          学号
          <input type="text" bind:value={localEdit.student_no}/>
        </label>
        <label>
          姓名
          <input type="text" bind:value={localEdit.name}/>
        </label>
        {#if editResult && editResult.type == "Conflict"}
          <div>存在冲突：学号：{editResult.data.student_no}，姓名：{editResult.data.name}
            ，创建于{format(editResult.data.created_at)}</div>
        {/if}
        <div class="dialog-actions">
          <button class="btn-secondary" onclick={close}>取消</button>
          <button onclick={edit}>保存</button>
        </div>
      {/if}
    </div>
  </div>
{/if}
