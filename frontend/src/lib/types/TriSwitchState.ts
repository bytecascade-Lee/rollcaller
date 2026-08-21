export enum TriSwitchState {
  Negative = -1,
  Neutral = 0,
  Positive = 1
}

export const STATE_CONFIG = {
  [-1]: {thumbLeft: 2, trackBg: 'var(--color-surface)', label: '左'},
  [0]: {thumbLeft: 14, trackBg: 'var(--color-warn)', label: '中'},
  [1]: {thumbLeft: 26, trackBg: 'var(--color-primary)', label: '右'}
}
