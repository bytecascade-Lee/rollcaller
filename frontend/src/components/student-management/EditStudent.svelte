<script lang="ts">
  import {studentStore} from "$stores/studentStore.svelte";
  import type {EntryRegistry} from "$routing/registry";
  import type {StudentOverlayName, StudentSingleUpdate, StudentTable} from "$types";
  import type {AppEntryPayload} from "$routing/app.entries.generated";
  import {AppEntry} from "$routing/app.entries.generated";
  import {StudentCommand} from "$commands"
  import {DateTimeUtils} from "$utils";
  import {appNavigate} from "$routing/appNavigate";

  let {overlays}: { overlays: EntryRegistry<StudentOverlayName, AppEntryPayload> } = $props();

  let targetId = $state<number | null>(null);
  let isVisible = $state(false);
  let draft = $state<{ student_no: string; name: string } | null>(null);
  let editResult = $state<StudentSingleUpdate | undefined>(undefined);
  let isSaving = $state(false);

  let target = $state<StudentTable | null>(null);

  let canSave = $derived(
    draft != null &&
    draft.student_no.trim() !== "" &&
    draft.name.trim() !== ""
  );

  /** 把派生出来的行拷进草稿 */
  $effect(() => {
    if (draft !== null) return;
    const row = target;
    if (row) draft = {student_no: row.student_no, name: row.name};
  });

  function open(id: number | undefined) {
    if (studentStore.students.length === 0) void studentStore.load();
    if (id == null) return;
    target = studentStore.get(id);
    draft = null;
    editResult = undefined;
    isSaving = false;
    isVisible = true;
  }

  function close() {
    isVisible = false;
    targetId = null;
    draft = null;
    editResult = undefined;
    isSaving = false;
  }

  function requestClose() {
    appNavigate.leave(AppEntry.editStudent);
  }

  async function doEdit() {
    if (!target || !draft || !canSave || isSaving) return;
    const student_no = draft.student_no.trim();
    const name = draft.name.trim();
    try {
      isSaving = true;
      editResult = await StudentCommand.update({id: target.id, student_no, name});
      if (editResult.type == "Update") {
        studentStore.upsert(editResult.data);
        requestClose();
      }
      // Conflict：保持弹窗打开，提示用户修改学号或姓名后重试
    } catch (e) {
      alert(String(e));
    } finally {
      isSaving = false;
    }
  }

  $effect(() => {
    overlays.register(AppEntry.editStudent, {
      open,
      close,
    })
    return () => overlays.unregister(AppEntry.editStudent);
  })
</script>

{#if isVisible}
  <!-- svelte-ignore a11y_click_events_have_key_events -->
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div class="overlay" onclick={requestClose}>
    <div class="popup" onclick={(e) => e.stopPropagation()}>
      <h3 class="text-title">修改学生</h3>
      {#if draft == null}
        <span class="text-content">
          {#if studentStore.isLoading}
            正在加载学生数据...
          {:else}
            未找到 id 为 {targetId} 的学生，可能已被删除
          {/if}
        </span>
        <div class="button-group">
          <button
            type="button"
            class="button"
            onclick={requestClose}
          >
            关闭
          </button>
        </div>
      {:else}
        <form onsubmit={(e) => { e.preventDefault(); doEdit(); }}>
          <div class="field">
            <span class="field-label">学号</span>
            <input
              type="text"
              bind:value={draft.student_no}
              oninput={() => { if (editResult) editResult = undefined; }}
            />
          </div>
          <div class="field">
            <span class="field-label">姓名</span>
            <input
              type="text"
              bind:value={draft.name}
              oninput={() => { if (editResult) editResult = undefined; }}
            />
          </div>
          {#if editResult && editResult.type == "Conflict"}
            <span class="text-subtitle error">学号已被占用<br/></span>
            <span class="text-content">
                学号「{editResult.data.student_no}」已被学生<b>{editResult.data.name}</b>使用<br/>
                （创建于 {DateTimeUtils.format(editResult.data.created_at)}）<br/>
                请修改学号后重试
              </span>
          {/if}
          <div class="button-group">
            <button
              type="button"
              class="button"
              onclick={requestClose}
              disabled={isSaving}
            >
              取消
            </button>
            <button
              type="submit"
              class="button yes"
              disabled={isSaving || !canSave}>
              {isSaving ? "保存中..." : "保存"}
            </button>
          </div>
        </form>
      {/if}
    </div>
  </div>
{/if}
