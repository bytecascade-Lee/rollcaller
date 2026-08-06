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
    <div class="popup" onclick={(e) => e.stopPropagation()}>
      <form onsubmit={(e) => { e.preventDefault(); create(null); }}>
        <h3 class="text-title">添加学生</h3>
        <label class="field">
          <span class="field-label">学号</span>
          <!-- svelte-ignore a11y_autofocus -->
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
            <span class="text-subtitle error">学号已被占用<br/></span>
            <span class="text-content">
                学号「{result.data.student_no}」已被学生<b>{result.data.name}</b>使用<br/>
                （创建于 {format(result.data.created_at)}）<br/>
                请修改学号后重试
              </span>
          {:else if result.type == "Conflict"}
            <span class="text-subtitle warn">学号冲突 — 存在已删除的记录<br/></span>
            <table>
              <thead>
              <tr>
                <th></th>
                <th>当前输入</th>
                <th>原记录</th>
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
            <span class="text-content">原记录已被删除，是否用新姓名覆盖并恢复？</span>
          {:else if result.type == "Restore"}
            <span class="text-subtitle success">已自动恢复<br/></span>
            <span class="text-content">
                学号「{result.data.student_no}」曾存在且已删除，系统已自动恢复原记录。
              </span>
          {:else if result.type == "Insert"}
            <span class="text-subtitle success">添加成功<br/></span>
            <span class="text-content">
                学生 <b>{result.data.name}</b>（{result.data
              .student_no}）已添加，可继续录入下一位。
              </span>
          {:else if result.type == "Override"}
            <span class="text-subtitle">已覆写<br/></span>
            <span class="text-content">
                学生「{result.data.student_no}」已用新姓名<br/>
                <b>{result.data.name}</b> 覆写并恢复。
              </span>
          {:else if result.type == "Retain"}
            <span class="text-subtitle">已保留原记录<br/></span>
            <span class="text-content">
                学号「{result.data.student_no}」的原记录未被修改<br/>
                请更换学号后重新添加。
              </span>
          {/if}
        {/if}

        <div class="button-group">
          <button
            type="button"
            class="button"
            onclick={close}
            disabled={isCreating}
          >
            取消
          </button>
          {#if result?.type == "Conflict"}
            <button
              type="button"
              class="button yes"
              onclick={() => create(false)}
              disabled={isCreating}
            >
              保留原记录
            </button>
            <button
              type="button"
              class="button warn"
              onclick={() => create(true)}
              disabled={isCreating}
            >
              {isCreating ? "处理中..." : "覆盖并恢复"}
            </button>
          {:else}
            <button
              type="submit"
              class="button yes"
              disabled={isCreating || !canSubmit}
            >
              {isCreating ? "提交中..." : "确定"}
            </button>
          {/if}
        </div>
      </form>
    </div>
  </div>
{/if}

<style>
    table {
      width: 100%;
      border-collapse: collapse;
    }

    th, td {
      border: var(--border-size-xs) solid var(--border-color-5);
      border-radius:  var(--radius-md);
      padding: var(--space-xs) var(--space-sm);
      text-align: center;
    }
</style>
