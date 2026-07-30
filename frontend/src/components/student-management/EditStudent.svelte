<script lang="ts">
  import {studentStore} from "$stores/studentStore.svelte";
  import {invoke} from "@tauri-apps/api/core";
  import {studentManagementDialogController} from "$services/studentManagementDialogController";


  let {selected = $bindable()} = $props();
  let isVisible = $state(false);
  let closeOnOutside = false;

  async function edit() {
    if (!selected || !selected.name.trim() || !selected.student_no.trim() || selected.length != 1) return;
    try {
      await invoke("update_student", {
        student: {
          id: selected.id,
          student_no: selected.student_no.trim(),
          name: selected.name.trim(),
        }
      }).then(() => {
        studentStore.upsert(selected);
      });
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
      <h3>修改学生</h3>
      <label>
        学号
        <input type="text" bind:value={selected.student_no}/>
      </label>
      <label>
        姓名
        <input type="text" bind:value={selected.name}/>
      </label>
      <div class="dialog-actions">
        <button class="btn-secondary" onclick={() => isVisible = false}>取消</button>
        <button onclick={edit}>保存</button>
      </div>
    </div>
  </div>
  {/if}}
