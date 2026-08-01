export type StudentManagementDialogName = "SingleCreate" | "Edit" | "Delete" | "Import" | "Export";

export type StudentManagementDialog = {
  open: () => void;
  close: () => void;
  isVisible?: () => boolean;
}

class StudentManagementDialogController {
  #controllers = new Map<StudentManagementDialogName, StudentManagementDialog>();

  controllers() {
    return this.#controllers
  }

  register(name: StudentManagementDialogName, dialog: StudentManagementDialog) {
    this.#controllers.set(name, dialog)
  }

  unregister(name: StudentManagementDialogName) {
    this.#controllers.delete(name)
  }

  open(name: StudentManagementDialogName) {
    this.#controllers.get(name)?.open()
  }

  close(name: StudentManagementDialogName) {
    this.#controllers.get(name)?.close()
  }

  isOpen(name: StudentManagementDialogName) {
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

export const studentManagementDialogController = new StudentManagementDialogController();
