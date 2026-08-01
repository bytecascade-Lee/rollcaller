export type Overlay = {
  open: () => void;
  close: () => void;
  isVisible?: () => boolean;
}
