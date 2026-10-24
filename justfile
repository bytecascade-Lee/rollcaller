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

# 启动完整 Tauri 开发环境，用法: just dev-target x86_64-pc-windows-msvc
dev target:
    {{ uv }} run python {{ scripts_dir }}/dev.py --target {{ target }}

# 构建

# 本地构建，用法: just build 0.8.3
build ver:
    {{ uv }} run python {{ scripts_dir }}/build.py -v {{ ver }} -m local

# 本地构建并指定目标三元组（用法: just build-target 0.8.3 aarch64-pc-windows-msvc）
build-target ver target:
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

# 检查 & 格式化

# 运行所有检查，前端 + 后端
check: check-fe check-be

# 前端类型检查
check-fe:
    cd {{ frontend_dir }} && pnpm check

# 后端编译检查
check-be:
    cd {{ backend_dir }} && cargo check

# 脚本工具

# 打发布 tag，用法: just bump patch / just bump prerelease --serial
bump *args:
    {{ uv }} run python {{ scripts_dir }}/bump.py {{ args }}

# 生成提交信息
gen-msg *args:
    {{ uv }} run python {{ scripts_dir }}/tool/generate_commit_message.py {{ args }}

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
