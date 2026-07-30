<script lang="ts">
  import {studentStore} from "$stores/studentStore.svelte";
  import type {Student} from "$types/Student";
  import type {StudentSingleCreateResult} from "$types/StudentSingleCreateResult";
  import {format} from "$utils/DataTimeUtils";
  import {invoke} from "@tauri-apps/api/core";
  import {studentManagementDialogController} from "$controllers/studentManagementDialogController";

  let newStudent = $state<Student>({
    id: null,
    student_no: "",
    name: "",
  });
  let addResult = $state<StudentSingleCreateResult | null>(null);
  let isCreating = $state(false);
  let isVisible = $state(false);
  let closeOnOutside = true;

  async function create(overwrite: boolean | null) {
    if (!newStudent.student_no.trim() || !newStudent.name.trim()) return;
    try {
      isCreating = true;
      await invoke<StudentSingleCreateResult>("create_student", {
        studentNo: newStudent.student_no.trim(),
        name: newStudent.name.trim(),
        overwrite: overwrite,
      }).then((result) => {
        studentStore.upsert(result.data);
        addResult = result;
        isCreating = false;
      });
    } catch (e) {
      alert(String(e));
      isCreating = false;
    }
  }

  $effect(() => {
    studentManagementDialogController.register("SingleCreate", {
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
      <h3>添加学生</h3>
      <label>学号<input
        type="text"
        bind:value={newStudent.student_no}
        placeholder="如 1097260001"
      /></label>
      <label>姓名<input
        type="text"
        bind:value={newStudent.name}
        placeholder="如 张三"
      /></label>
      {#if addResult != null}
        {#if addResult.type == "ActiveExists"}
          <div class="msg-box msg-warn">
            <strong>学号已被占用</strong>
            <p>
              学号「{addResult.data.student_no}」已被学生
              <b>{addResult.data.name}</b>
              使用<br/>
              （创建于 {format(addResult.data.created_at)}）。
            </p>
            <p>请修改学号或姓名后重试。</p>
          </div>
        {:else if addResult.type == "Conflict"}
          <div class="msg-box msg-conflict">
            <strong>学号冲突 — 存在已删除的记录</strong>
            <table class="diff-table">
              <thead>
              <tr>
                <th></th>
                <th>当前输入</th>
                <th>原记录（已删除）</th>
              </tr>
              </thead>
              <tbody>
              <tr>
                <td>学号</td>
                <td>{newStudent.student_no}</td>
                <td>{addResult.data.student_no}</td>
              </tr>
              <tr>
                <td>姓名</td>
                <td><b>{newStudent.name}</b></td>
                <td><b>{addResult.data.name}</b></td>
              </tr>
              </tbody>
            </table>
            <p>原记录已被软删除，是否用新姓名覆盖并恢复？</p>
          </div>
        {:else if addResult.type == "Restore"}
          <div class="msg-box msg-info">
            <strong>已自动恢复</strong>
            <p>
              学号「{addResult.data.student_no}」曾存在且已删除，系统已自动恢复原记录。
            </p>
          </div>
        {:else if addResult.type == "Insert"}
          <div class="msg-box msg-success">
            <strong>添加成功</strong>
            <p>
              学生 <b>{addResult.data.name}</b>（{addResult.data
              .student_no}）已添加。
            </p>
          </div>
        {:else if addResult.type == "Override"}
          <div class="msg-box msg-info">
            <strong>已覆写</strong>
            <p>
              学生「{addResult.data.student_no}」已用新姓名
              <b>{addResult.data.name}</b> 覆写并恢复。
            </p>
          </div>
        {/if}
      {/if}

      <div class="dialog-actions">
        <button class="btn-secondary" onclick={() => isVisible = false}>取消</button>
        {#if addResult != null && addResult.type == "Conflict"}
          <button onclick={() =>create(true)} disabled={isCreating}>覆盖并恢复</button>
        {:else}
          <button onclick={() =>create(null)} disabled={isCreating}>确定</button>
        {/if}
      </div>
    </div>
  </div>
{/if}
