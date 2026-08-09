import {PopupName} from "$types/PopupName";
import {Popup} from "$types/Popup";

class PopupController {
  #controllers = new Map<PopupName, Popup>();

  controllers() {
    return this.#controllers
  }

  register(name: PopupName, dialog: Popup) {
    this.#controllers.set(name, dialog)
  }

  unregister(name: PopupName) {
    this.#controllers.delete(name)
  }

  open(name: PopupName) {
    this.#controllers.get(name)?.open()
  }

  close(name: PopupName) {
    this.#controllers.get(name)?.close()
  }

  isOpen(name: PopupName) {
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

export const overlayController = new PopupController();
