<script lang="ts">
  import { studentStore } from "$stores/studentStore.svelte";
  import { invoke } from "@tauri-apps/api/core";

  let { selectIds = $bindable() } = $props<bigint[]>();
  let isVisible = $state(false);
  let closeOnOutside = true;

  async function del() {
    try {
      await invoke("delete_students", {
        ids: selectIds,
      }).then(() => {
        studentStore.remove(selectIds);
        // [FIXME]: 此处需要使用回调，直接赋值会导致 bind 失效。
        selectIds = [];
      });
    } catch (e) {
      alert(String(e));
    }
  }

  export function open() {
    isVisible = true;
  }

  export function close() {
    isVisible = false;
  }
</script>

{#if isVisible}
  <!-- svelte-ignore a11y_click_events_have_key_events -->
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div class="overlay" onclick={closeOnOutside ? close : undefined}>
    <div class="dialog" onclick={(e) => e.stopPropagation()}>
      <h3>确认删除</h3>
      <p>确定删除选中的 {selectIds.length} 条记录吗？</p>
      <div class="dialog-actions">
        <button class="btn-secondary" onclick={close}>取消</button>
        <button class="btn-danger" onclick={del}>删除</button>
      </div>
    </div>
  </div>
{/if}
