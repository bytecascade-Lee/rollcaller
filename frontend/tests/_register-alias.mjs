/**
 * 只做一件事：把上面的解析钩子登记给 Node（`--import` 需要的是一个"副作用模块"，
 * 钩子本身要经 `register()` 装载，所以拆成两个文件）。
 *
 * 用法见 `package.json` 的 `test:help` 脚本。
 */
import { register } from "node:module";

register("./_alias-resolver.mjs", import.meta.url);
