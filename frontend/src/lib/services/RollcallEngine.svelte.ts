import {RollcallPhase, RollcallEvent} from "$types";
import {RecordCommand, RollcallCommand} from "$commands";
import type {Record, RollcallRecord} from "$types";
import {studentStore} from "$stores/studentStore.svelte";
import {recordStore} from "$stores/recordStore.svelte";
import {uuid} from "$utils/UuidUtils";

/** 名字滚动切换间隔（ms） */
const ROLL_INTERVAL = 120;
/** 连续点名时动画自动结束时长（ms） */
const ANIMATE_DURATION = 720;
/** 结果展示时长（ms） */
const SHOW_DURATION = 1200;

/**
 * 事件驱动下的点名状态机
 *
 * 转换规则：
 * - Idle      + Start       → Animating    重置会话，启动滚动动画
 * - Animating + AnimateDone → Picking      停止动画，随机选人
 * - Animating + UserStop    → Picking      强制，设置 pendingStop
 * - Picking   + UserStop    →              忽略（选人+存库事务必须走完）
 * - Picking   + PickDone    → Showing      展示名字，写入数据库
 * - Showing   + SaveSuccess →              更新 recordStore，completedTimes+1
 * - Showing   + SaveFailed  → Idle         撤销展示的名字，提示错误
 * - Showing   + ShowDone    → pendingStop  或已完成次数≥总次数 → Idle；否则 → Animating
 * - Showing   + UserStop    →              仅设置 pendingStop，不影响当前计时
 */
class RollcallEngine {
  phase = $state(RollcallPhase.Idle);
  currentName = $state<string | null>(null);
  totalTimes = $state(1);
  completedTimes = $state(0);
  allowRepetition = $state(false);
  called = $state<bigint[]>([]);

  /** 用户暂停标记：不打断当前事务，仅在自然中断点（ShowDone）生效 */
  #pendingStop = $state(false);
  #sessionId = $state("");
  // @ts-ignore
  #animTimer: NodeJS.Timeout | null = null;
  // @ts-ignore
  #animTimeout: NodeJS.Timeout | null = null;
  // @ts-ignore
  #showTimeout: NodeJS.Timeout | null = null;
  #savedRecord: RollcallRecord | null = null;

  isRolling = $derived(this.phase !== RollcallPhase.Idle);

  /** 更新点名次数（至少为 1） */
  updateTotalTimes(n: number) {
    const value = Math.floor(n);
    this.totalTimes = Number.isFinite(value) && value > 0 ? value : 1;
  }

  /** 开始点名 / 停止点名 */
  toggle() {
    if (this.phase === RollcallPhase.Idle) {
      if (studentStore.students.length === 0) {
        alert("没有可点名的学生，请先添加学生");
        return;
      }
      // 重置会话
      this.#sessionId = uuid();
      this.completedTimes = 0;
      this.#pendingStop = false;
      this.currentName = null;
      this.#dispatch(RollcallEvent.Start);
    } else {
      this.#dispatch(RollcallEvent.UserStop);
    }
  }

  #dispatch(event: RollcallEvent) {
    switch (this.phase) {
      case RollcallPhase.Idle:
        if (event === RollcallEvent.Start) {
          // 单次点名：等待用户点击停止；连续点名：动画自动推进
          this.#enterAnimating(this.totalTimes > 1);
        }
        break;

      case RollcallPhase.Animating:
        if (event === RollcallEvent.AnimateDone || event === RollcallEvent.UserStop) {
          if (event === RollcallEvent.UserStop) this.#pendingStop = true;
          this.#clearAnim();
          this.phase = RollcallPhase.Picking;
          void this.#runPicking();
        }
        break;

      case RollcallPhase.Picking:
        // 选人 + 存库事务进行中，UserStop 忽略
        break;

      case RollcallPhase.Showing:
        if (event === RollcallEvent.SaveSuccess) {
          if (this.#savedRecord) recordStore.upsert(this.#savedRecord);
          this.completedTimes++;
          this.#showTimeout = setTimeout(
            () => this.#dispatch(RollcallEvent.ShowDone),
            SHOW_DURATION
          );
        } else if (event === RollcallEvent.SaveFailed) {
          this.#undoShow();
        } else if (event === RollcallEvent.UserStop) {
          this.#pendingStop = true;
        } else if (event === RollcallEvent.ShowDone) {
          this.#showTimeout = null;
          if (this.#pendingStop || this.completedTimes >= this.totalTimes) {
            this.#pendingStop = false;
            this.phase = RollcallPhase.Idle;
          } else {
            // 连续点名：进入下一轮动画
            this.#enterAnimating(true);
          }
        }
        break;
    }
  }

  #enterAnimating(autoAdvance: boolean) {
    this.phase = RollcallPhase.Animating;
    this.currentName = null;
    this.#animTimer = setInterval(() => {
      if (studentStore.students.length > 0) {
        const idx = Math.floor(Math.random() * studentStore.students.length);
        this.currentName = studentStore.students[idx].name;
      }
    }, ROLL_INTERVAL);
    if (autoAdvance) {
      this.#animTimeout = setTimeout(
        () => this.#dispatch(RollcallEvent.AnimateDone),
        ANIMATE_DURATION
      );
    }
  }

  #clearAnim() {
    if (this.#animTimer) {
      clearInterval(this.#animTimer);
      this.#animTimer = null;
    }
    if (this.#animTimeout) {
      clearTimeout(this.#animTimeout);
      this.#animTimeout = null;
    }
  }

  /** Picking：随机选人 → 展示名字 → 写入数据库，事务必须完整走完 */
  async #runPicking() {
    try {
      if (this.called.length == studentStore.students.length) {
        this.called = [];
      }
      let ids = studentStore.students.map((s) => s.id);
      if (!this.allowRepetition) {
        ids = ids.filter((id) => !this.called.includes(id));
      }
      const studentId = await RollcallCommand.pick(ids);
      this.called = [...this.called, studentId];
      const student = studentStore.students.find((s) => s.id === studentId);

      // PickDone → Showing：展示学生名字 + 显式写入数据库
      this.phase = RollcallPhase.Showing;
      this.currentName = student?.name ?? "未知学生";
      const record: Record = {
        id: null,
        student_id: studentId,
        attendance_status: 1, // 出勤
        remark: null,
        rollcall_at: Date.now(),
        session_id: this.#sessionId,
      };
      this.#savedRecord = await RecordCommand.create(record);
      this.#dispatch(RollcallEvent.SaveSuccess);
    } catch (e) {
      // SaveFailed：撤销展示的名字，重置状态
      this.#savedRecord = null;
      this.#dispatch(RollcallEvent.SaveFailed);
      alert("点名保存失败：" + e);
    }
  }

  #undoShow() {
    this.#pendingStop = false;
    this.currentName = "等待点名";
    this.phase = RollcallPhase.Idle;
  }
}

export const rollcallEngine = new RollcallEngine();
