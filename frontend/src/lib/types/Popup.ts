export type Popup = {
  open: () => void;
  close: () => void;
  isVisible?: () => boolean;
}
