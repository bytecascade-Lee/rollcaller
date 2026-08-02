<script lang="ts">
  import {studentStore} from "$stores/studentStore.svelte";
  import type {Student, StudentSingleCreateResult} from "$types";
  import {format} from "$utils/DataTimeUtils";
  import {StudentCommand} from "$commands"
  import {overlayController} from "$controllers/overlayController";

  let newStudent = $state<Student>({
    id: null,
    student_no: "",
    name: "",
  });
  let result = $state<StudentSingleCreateResult | null>(null);
  let isCreating = $state(false);
  let isVisible = $state(false);

  let canSubmit = $derived(
    newStudent.student_no.trim() !== "" && newStudent.name.trim() !== ""
  );

  function resetForm() {
    result = null;
    isCreating = false;
  }

  function open() {
    resetForm();
    isVisible = true;
  }

  function close() {
    isVisible = false;
    resetForm();
  }

  async function create(override: boolean | null) {
    newStudent.student_no = newStudent.student_no.trim();
    newStudent.name = newStudent.name.trim();
    if (!canSubmit || isCreating) return;

    try {
      isCreating = true;
      result = await StudentCommand.create({...newStudent}, override);
      switch (result.type) {
        case "Insert":
        case "Restore":
        case "Override":
          // 写入成功：更新列表，清空输入，便于连续录入下一位学生
          studentStore.upsert(result.data);
          newStudent.student_no = "";
          newStudent.name = "";
          break;
        case "Retain":
          // 用户选择保留原（已删除）记录：该学号不可再用，清空学号引导换号重录
          newStudent.student_no = "";
          break;
        case "ActiveExists":
        case "Conflict":
          // 保留输入，交由用户修改学号或做出决策
          break;
      }
    } catch (e) {
      alert(String(e));
    } finally {
      isCreating = false;
    }
  }

  $effect(() => {
    overlayController.register("StudentSingleCreate", {
      open: open,
      close: close,
      isVisible: () => isVisible
    })
  })
</script>

{#if isVisible}
  <!-- svelte-ignore a11y_click_events_have_key_events -->
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <!-- 点击遮罩不可关闭，确保不会误触 -->
  <div class="overlay">
    <div class="dialog" onclick={(e) => e.stopPropagation()}>
      <form onsubmit={(e) => { e.preventDefault(); create(null); }}>
        <h3>添加学生</h3>
        <label class="field">
          <span class="field-label">学号</span>
          <input
            type="text"
            bind:value={newStudent.student_no}
            placeholder="如 1097260001"
            autofocus
          />
        </label>
        <label class="field">
          <span class="field-label">姓名</span>
          <input
            type="text"
            bind:value={newStudent.name}
            placeholder="如 张三"
          />
        </label>
        {#if result != null}
          {#if result.type == "ActiveExists"}
            <div class="msg" data-kind="warn">
              <strong>学号已被占用</strong>
              <p>
                学号「{result.data.student_no}」已被学生
                <b>{result.data.name}</b>
                使用<br/>
                （创建于 {format(result.data.created_at)}）。
              </p>
              <p>请修改学号后重试。</p>
            </div>
          {:else if result.type == "Conflict"}
            <div class="msg" data-kind="warn">
              <strong>学号冲突 — 存在已删除的记录</strong>
              <table>
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
                  <td>{result.data.student_no}</td>
                </tr>
                <tr>
                  <td>姓名</td>
                  <td><b>{newStudent.name}</b></td>
                  <td><b>{result.data.name}</b></td>
                </tr>
                </tbody>
              </table>
              <p>原记录已被软删除，是否用新姓名覆盖并恢复？</p>
            </div>
          {:else if result.type == "Restore"}
            <div class="msg" data-kind="info">
              <strong>已自动恢复</strong>
              <p>
                学号「{result.data.student_no}」曾存在且已删除，系统已自动恢复原记录。
              </p>
            </div>
          {:else if result.type == "Insert"}
            <div class="msg" data-kind="success">
              <strong>添加成功</strong>
              <p>
                学生 <b>{result.data.name}</b>（{result.data
                .student_no}）已添加，可继续录入下一位。
              </p>
            </div>
          {:else if result.type == "Override"}
            <div class="msg" data-kind="info">
              <strong>已覆写</strong>
              <p>
                学生「{result.data.student_no}」已用新姓名
                <b>{result.data.name}</b> 覆写并恢复。
              </p>
            </div>
          {:else if result.type == "Retain"}
            <div class="msg" data-kind="info">
              <strong>已保留原记录</strong>
              <p>
                学号「{result.data.student_no}」的原记录（已删除）未被修改，
                请更换学号后重试。
              </p>
            </div>
          {/if}
        {/if}

        <div class="button-group">
          <button
            type="button"
            class="btn"
            style:--btn-bg="var(--app-color-surface-muted)"
            style:--btn-color="var(--app-color-text)"
            onclick={close}
            disabled={isCreating}
          >取消</button>
          {#if result?.type == "Conflict"}
            <button
              type="button"
              class="btn"
              style:--btn-bg="var(--app-color-surface-muted)"
              style:--btn-color="var(--app-color-text)"
              onclick={() => create(false)}
              disabled={isCreating}
            >保留原记录</button>
            <button type="button" class="btn" onclick={() => create(true)} disabled={isCreating}>
              {isCreating ? "处理中..." : "覆盖并恢复"}
            </button>
          {:else}
            <button type="submit" class="btn" disabled={isCreating || !canSubmit}>
              {isCreating ? "提交中..." : "确定"}
            </button>
          {/if}
        </div>
      </form>
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
    background: var(--msg-bg, var(--app-color-surface-muted));
    color: var(--msg-color, var(--app-color-text-muted));
    text-align: left;
  }

  .msg p {
    margin: 0;
  }

  .msg[data-kind="success"] {
    --msg-bg: var(--green-1);
    --msg-color: var(--green-9);
  }

  .msg[data-kind="warn"] {
    --msg-bg: var(--orange-1);
    --msg-color: var(--orange-9);
  }

  .msg[data-kind="info"] {
    --msg-bg: var(--blue-1);
    --msg-color: var(--blue-9);
  }
</style>
