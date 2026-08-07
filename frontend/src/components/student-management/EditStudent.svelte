<script lang="ts">
  import {studentStore} from "$stores/studentStore.svelte";
  import {overlayController} from "$controllers/overlayController";
  import type {StudentSingleUpdate, StudentTable} from "$types";
  import {StudentCommand} from "$commands"
  import {format} from "$utils/DataTimeUtils";

  let {selected = $bindable()}: { selected: Set<bigint> } = $props();
  let localEdit = $state<StudentTable | null>(null);
  let isVisible = $state(false);
  let closeOnOutside = true;
  let editResult = $state<StudentSingleUpdate | undefined>(undefined);
  let isSaving = $state(false);

  let canSave = $derived(
    localEdit != null &&
    localEdit.student_no.trim() !== "" &&
    localEdit.name.trim() !== ""
  );

  async function edit() {
    if (!localEdit || !canSave || isSaving) return;
    localEdit.student_no = localEdit.student_no.trim();
    localEdit.name = localEdit.name.trim();
    try {
      isSaving = true;
      editResult = await StudentCommand.update({
        id: localEdit.id,
        student_no: localEdit.student_no,
        name: localEdit.name,
      });
      if (editResult.type == "Update") {
        studentStore.upsert(editResult.data);
        close();
      }
      // Conflict：保持弹窗打开，提示用户修改学号或姓名后重试
    } catch (e) {
      alert(String(e));
    } finally {
      isSaving = false;
    }
  }

  function close() {
    isVisible = false;
    localEdit = null;
    editResult = undefined;
    isSaving = false;
  }

  function open() {
    editResult = undefined;
    isVisible = true;
  }

  $effect(() => {
    if (isVisible && selected.size == 1) {
      // 必须拷贝，否则拿到的是引用，表格中的照样会变
      let value = selected.values().next().value;
      const original = studentStore.get(value ? value : -1n);
      localEdit = original ? {...original} : null;
    }
  });
  $effect(() => {
    overlayController.register("StudentEdit", {
      open: open,
      close: close,
      isVisible: () => isVisible
    })
  })
</script>

{#if isVisible}
  <!-- svelte-ignore a11y_click_events_have_key_events -->
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div class="overlay" onclick={closeOnOutside ? () => close() : undefined}>
    <div class="popup" onclick={(e) => e.stopPropagation()}>
      {#if selected.size > 1}
        <h3 class="text-title error">仅支持单个修改<br/></h3>
        <span class="text-content">当前选中了 {selected.size} 名学生，请只选择一名后再试</span>
      {:else if localEdit == null}
        <h3 class="text-title error">未找到待编辑的学生<br/></h3>
        <span class="text-content">当前选中id：{Array.from(selected)}</span>
      {:else}
        <form onsubmit={(e) => { e.preventDefault(); edit(); }}>
          <h3 class="text-title">修改学生</h3>
          <label class="field">
            <span class="field-label">学号</span>
            <input
              type="text"
              bind:value={localEdit.student_no}
              oninput={() => { if (editResult) editResult = undefined; }}
            />
          </label>
          <label class="field">
            <span class="field-label">姓名</span>
            <input
              type="text"
              bind:value={localEdit.name}
              oninput={() => { if (editResult) editResult = undefined; }}
            />
          </label>
          {#if editResult && editResult.type == "Conflict"}
            <span class="text-subtitle error">学号已被占用<br/></span>
            <span class="text-content">
                学号「{editResult.data.student_no}」已被学生<b>{editResult.data.name}</b>使用<br/>
                （创建于 {format(editResult.data.created_at)}）<br/>
                请修改学号后重试
              </span>
          {/if}
          <div class="button-group">
            <button
              type="button"
              class="button"
              onclick={close}
              disabled={isSaving}
            >
              取消
            </button>
            {#if selected.size == 1 && localEdit != null}
              <button
                type="submit"
                class="button yes"
                disabled={isSaving || !canSave}>
                {isSaving ? "保存中..." : "保存"}
              </button>
            {/if}
          </div>
        </form>
      {/if}
    </div>
  </div>
{/if}
