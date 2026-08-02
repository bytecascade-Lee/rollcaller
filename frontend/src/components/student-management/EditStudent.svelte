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
        studentStore.upsert(localEdit);
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
    <div class="dialog" onclick={(e) => e.stopPropagation()}>
      {#if selected.size > 1}
        <h3>仅支持单个修改</h3>
        <p>当前选中了 {selected.size} 名学生，请只选择一名后再试。</p>
        <div class="button-group">
          <button type="button" class="btn" onclick={close}>确定</button>
        </div>
      {:else if localEdit == null}
        <h3>未找到待编辑的学生</h3>
        <p>当前选中id：{Array.from(selected)}</p>
        <div class="button-group">
          <button type="button" class="btn" onclick={close}>确定</button>
        </div>
      {:else}
        <form onsubmit={(e) => { e.preventDefault(); edit(); }}>
          <h3>修改学生</h3>
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
            <div class="msg">
              <strong>学号已被占用</strong>
              <p>
                学号「{editResult.data.student_no}」已被学生
                <b>{editResult.data.name}</b>
                使用（创建于 {format(editResult.data.created_at)}）。
              </p>
              <p>请修改学号或姓名后重新保存。</p>
            </div>
          {/if}
          <div class="button-group">
            <button
              type="button"
              class="btn"
              style:--btn-bg="var(--app-color-surface-muted)"
              style:--btn-color="var(--app-color-text)"
              onclick={close}
              disabled={isSaving}
            >取消</button>
            <button type="submit" class="btn" disabled={isSaving || !canSave}>
              {isSaving ? "保存中..." : "保存"}
            </button>
          </div>
        </form>
      {/if}
    </div>
  </div>
{/if}

<style>
  .btn {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    min-height: var(--app-size-control);
    padding: var(--app-space-xs) var(--app-space-md);
    border: none;
    border-radius: var(--app-radius-sm);
    background: var(--btn-bg, var(--app-color-primary));
    color: var(--btn-color, var(--sand-0));
    font-family: inherit;
    font-size: var(--app-font-size-sm);
    font-weight: var(--app-font-weight-medium);
    cursor: pointer;
    transition: filter 150ms var(--app-ease), opacity 150ms var(--app-ease);
  }

  .btn:hover {
    filter: brightness(.94);
  }

  .btn:disabled {
    opacity: var(--app-opacity-disabled);
    cursor: not-allowed;
  }

  .msg {
    display: flex;
    flex-direction: column;
    gap: var(--app-space-xs);
    padding: var(--app-space-sm) var(--app-space-md);
    border-radius: var(--app-radius-sm);
    font-size: var(--app-font-size-sm);
    background: var(--app-color-surface-muted);
    color: var(--app-color-text-muted);
    text-align: left;
  }

  .msg p {
    margin: 0;
  }
</style>
