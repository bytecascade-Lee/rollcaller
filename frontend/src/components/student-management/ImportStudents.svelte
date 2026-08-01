<script lang="ts">
  import {studentStore} from "$stores/studentStore.svelte";
  import type {ImportPreviewData} from "$types/ImportPreviewData";
  import type {StudentBatchCreateResult} from "$types/StudentBatchCreateResult";
  import {invoke} from "@tauri-apps/api/core";
  import {open} from "@tauri-apps/plugin-dialog";
  import {overlayController} from "$controllers/overlayController";

  let previewData = $state<ImportPreviewData>({
    rows: [[]],
    total_rows: 0,
    total_columns: 0,
  });
  let filePath = $state("");
  let studentNoColumnIndex = $state(0);
  let nameColumnIndex = $state(0);
  let headerRows = $state(0);
  let isVisible = $state(false);
  let closeOnOutside = true;

  async function openFileDialog() {
    await open({
      multiple: false,
      filters: [{ name: "Excel", extensions: ["xlsx", "xls"] }],
    }).then((selected) => {
      if (!selected) return;
      filePath = selected;
    });
  }

  async function preview() {
    try {
      invoke<ImportPreviewData>("preview_excel", {
        filePath: filePath,
      }).then((result) => {
        previewData = result;
      });
    } catch (e) {
      alert("预览失败：" + e);
    }
  }

  async function importStudents() {
    try {
      invoke<StudentBatchCreateResult>("import_excel", {
        filePath: filePath,
        hearder_rows: headerRows,
        column_mapping: {
          student_no: studentNoColumnIndex,
          name: nameColumnIndex,
        },
        decisions: {},
      }).then((result) => {
        if (result.type === "Insert") {
          studentStore.load();
          alert(`成功导入 ${result.data.length} 名学生`);
        } else if (result.type === "Upsert") {
          studentStore.load();
          alert(`成功导入 ${result.data.length} 名学生（含恢复/覆写）`);
        } else if (result.type === "DuplicateInput") {
          alert(`导入数据中存在重复学号：${result.data.join("、")}`);
        } else if (result.type === "DecisionRequired") {
          alert(
            "部分学号存在已删除记录且姓名不同，请先处理冲突（后续版本支持交互式处理）",
          );
        } else if (result.type === "Conflict") {
          alert("部分学号已存在活跃记录，无法导入");
        }
      });
    } catch (e) {
      alert("导入失败：" + e);
    }
  }

  $effect(() => {
    overlayController.register("StudentImport", {
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
      <h3>导入预览 — 配置列映射</h3>

      <!-- 预览数据表格 -->
      <div class="preview-table-wrap">
        <table class="preview-table">
          <thead>
            <tr>
              {#each previewData.rows[0] as _, columnIndex}
                <th>
                  {#if columnIndex === studentNoColumnIndex}<span class="col-tag-no"
                      >学号</span
                    >{/if}
                  {#if columnIndex === nameColumnIndex}<span class="col-tag-name"
                      >姓名</span
                    >{/if}
                </th>
              {/each}
            </tr>
          </thead>
          <tbody>
            {#each previewData.rows as row}
              <tr>
                {#each row as cell}
                  <td>{cell}</td>
                {/each}
              </tr>
            {/each}
          </tbody>
        </table>
      </div>
      <p class="preview-info">
        共 {previewData.total_rows} 行 × {previewData.total_columns}
        列（显示前 {previewData.rows.length} 行）
      </p>

      <!-- 配置 -->
      <div class="config-grid">
        <label
          >表头行数
          <input
            type="number"
            min="0"
            max={previewData.total_rows - 1}
            bind:value={headerRows}
          />
        </label>
        <label
          >学号列索引 (0‑based)
          <input
            type="number"
            min="0"
            max={previewData.total_columns - 1}
            bind:value={studentNoColumnIndex}
          />
        </label>
        <label
          >姓名列索引 (0‑based)
          <input
            type="number"
            min="0"
            max={previewData.total_columns - 1}
            bind:value={nameColumnIndex}
          />
        </label>
      </div>

      <div class="dialog-actions">
        <button class="btn-secondary" onclick={() => isVisible = false}>关闭</button>
        <button onclick={importStudents}>导入</button>
      </div>
    </div>
  </div>
{/if}
