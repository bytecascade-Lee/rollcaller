<script lang="ts">
  import {statusText} from "$constants/AttendanceStatus";

  let {code}: { code: number } = $props();
</script>

<span class="status-badge" data-status={code}>
  {statusText(code)}
</span>

<style>
  .status-badge {
    display: inline-block;
    padding: 2px 10px;
    border-radius: 12px;
    font-size: 13px;
    font-weight: 500;
    background: var(--status-bg, #e5e7eb);
    color: var(--status-color, #374151);
  }

  /* 状态颜色 */
  .status-badge[data-status="0"] {
    --status-bg: #dc2626;
    --status-color: #fef2f2;
  }

  .status-badge[data-status="1"] {
    --status-bg: #16a34a;
    --status-color: #fef2f2;
  }

  .status-badge[data-status="2"] {
    --status-bg: #d97706;
    --status-color: #fffbeb;
  }

  .status-badge[data-status="3"] {
    --status-bg: #F1E710;
    --status-color: #1e1d1c;
  }

  .status-badge[data-status="4"] {
    --status-bg: #2563eb;
    --status-color: #eff6ff;
  }
</style>
