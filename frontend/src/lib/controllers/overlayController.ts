import {OverlayName} from "$types/OverlayName";
import {Overlay} from "$types/Overlay";

class OverlayController {
  #controllers = new Map<OverlayName, Overlay>();

  controllers() {
    return this.#controllers
  }

  register(name: OverlayName, dialog: Overlay) {
    this.#controllers.set(name, dialog)
  }

  unregister(name: OverlayName) {
    this.#controllers.delete(name)
  }

  open(name: OverlayName) {
    this.#controllers.get(name)?.open()
  }

  close(name: OverlayName) {
    this.#controllers.get(name)?.close()
  }

  isOpen(name: OverlayName) {
    return this.#controllers.get(name)?.isVisible?.() || false;
  }

  getKeys() {
    return Array.from(this.#controllers.keys())
  }

  openAll() {
    this.getKeys().forEach(v => this.open(v))
  }

  closeAll() {
    this.getKeys().forEach(v => this.close(v))
  }
}

export const overlayController = new OverlayController();
