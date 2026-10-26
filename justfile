set windows-shell := ["pwsh", "-NoLogo", "-NoProfile", "-Command"]

# 变量
uv := "uv"
frontend_dir := "frontend"
backend_dir := "backend"
scripts_dir := "scripts"

# 列出所有可用命令
default:
    @just --list

# 环境

# 初始化开发环境
setup:
    {{ uv }} run python {{ scripts_dir }}/setup.py

# 检查开发环境，不修改任何东西
doctor:
    {{ uv }} run python {{ scripts_dir }}/setup.py --check

# 开发

# 启动完整 Tauri 开发环境，用法: just dev x86_64-pc-windows-msvc
dev target:
    {{ uv }} run python {{ scripts_dir }}/dev.py -t {{ target }}

# 仅启动前端开发服务器（不跑 Tauri）
dev-fe:
    cd {{ frontend_dir }} && pnpm dev

# 构建

# 本地构建，用法: just build 0.8.3 x86_64-pc-windows-msvc
build ver target:
    {{ uv }} run python {{ scripts_dir }}/build.py -v {{ ver }} -t {{ target }} -m local

# 本地出包后驻留更新服务，需先 build 同一版本
release ver:
    {{ uv }} run python {{ scripts_dir }}/release.py -v {{ ver }} -m local

# 仅构建前端
build-fe:
    cd {{ frontend_dir }} && pnpm build

# 预览前端构建产物
preview:
    cd {{ frontend_dir }} && pnpm preview

# 测试

# 运行前后端测试，用法: just test x86_64-pc-windows-msvc
test target:
    {{ uv }} run python {{ scripts_dir }}/test.py -t {{ target }}

# 仅运行前端单元测试（后端测试请用 just test，它还会先刷新生成物）
test-fe:
    cd {{ frontend_dir }} && pnpm test

# 检查 & 格式化

# 运行所有检查，前端 + 后端，用法: just check x86_64-pc-windows-msvc
check target: check-fe (check-be target)

# 前端类型检查
check-fe:
    cd {{ frontend_dir }} && pnpm check

# 后端编译检查
check-be target:
    cd {{ backend_dir }} && cargo check --target {{ target }}

# 后端格式化
fmt:
    cd {{ backend_dir }} && cargo fmt

# 脚本工具

# 打发布 tag，用法: just bump patch / just bump prerelease --serial
bump *args:
    {{ uv }} run python {{ scripts_dir }}/bump.py {{ args }}

# 生成提交信息
gen-msg *args:
    {{ uv }} run python {{ scripts_dir }}/tool/generate_commit_message.py {{ args }}

# 导出 git 变更到 docs/diff（每个文件一个 md），用法: just export backend/src --from HEAD~1
export *args:
    {{ uv }} run python {{ scripts_dir }}/tool/export_git_diff.py {{ args }}

# 清理

# 清理后端构建产物
clean-be:
    cd {{ backend_dir }} && cargo clean

# 清理前端构建产物
[windows]
clean-fe:
    Remove-Item -Recurse -Force {{ frontend_dir }}/dist, {{ frontend_dir }}/node_modules/.vite

[unix]
clean-fe:
    rm -rf {{ frontend_dir }}/dist {{ frontend_dir }}/node_modules/.vite

# 清理所有构建产物
clean: clean-be clean-fe
