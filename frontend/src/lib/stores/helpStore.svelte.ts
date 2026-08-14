import {HelpCommand} from "$commands";

/** 特殊节点走独立命令，其余叶子走 help_load_markdown(id)。 */
const SPECIAL_LOADERS: Record<string, () => Promise<string>> = {
  readme: HelpCommand.readme,
  license: HelpCommand.license,
  changelog: HelpCommand.changelog,
  release_notes: HelpCommand.releaseNotes,
};

class HelpStore {
  content = $state("");

  #cache = new Map<string, string>();

  /** 按 id 加载内容：命中缓存直接返回，否则经分发表拉取并缓存。 */
  async load(id: string) {
    if (!this.#cache.has(id)) {
      const loader = SPECIAL_LOADERS[id] ?? (() => HelpCommand.markdown(id));
      this.#cache.set(id, await loader());
    }
    this.content = this.#cache.get(id)!;
  }
}

export const helpStore = new HelpStore();
