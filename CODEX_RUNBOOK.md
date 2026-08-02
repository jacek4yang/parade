# CODEX_RUNBOOK.md

## 目标

把本压缩包中的 Markdown 文件放到当前 Parade 仓库根目录，然后让已经登录 GitHub 的 Linux Codex CLI 自动完成：

1. 审计当前代码；
2. 建立实施计划；
3. 重构安全架构；
4. 实现手工流量基数、持续累加和周期自动重置；
5. 实现资源、进程、网络和安全分析；
6. 重做美观界面；
7. 测试；
8. 创建功能分支；
9. 提交并推送 GitHub；
10. 创建 Draft Pull Request。

## 文件说明

- `AGENTS.md`：Codex 每次进入仓库时使用的长期硬约束。
- `CODEX_GOAL.md`：本次完整开发目标。
- `TRAFFIC_ACCOUNTING_SPEC.md`：流量计算的规范算法。
- `UI_SPEC.md`：界面与交互规范。
- `CODEX_RUNBOOK.md`：当前操作手册。

## 第一步：解压到仓库根目录

你的仓库路径是：

```bash
cd ~/src/parade
```

把 ZIP 上传到服务器后执行：

```bash
cd ~/src/parade
unzip -o parade-codex-linux-spec.zip
```

确认：

```bash
ls -la AGENTS.md CODEX_GOAL.md TRAFFIC_ACCOUNTING_SPEC.md UI_SPEC.md CODEX_RUNBOOK.md
```

不要把压缩包解压成额外的一层目录。上述文件必须直接位于 `~/src/parade` 根目录。

## 第二步：检查 Git 状态

在普通 Shell 中执行：

```bash
cd ~/src/parade
git status --short --branch
git remote -v
gh auth status
```

如果当前已有未提交修改，不要盲目清理。让 Codex 识别并保留这些修改。

## 第三步：重新启动 Codex

由于 `AGENTS.md` 是项目级指令，最稳妥的方式是在文件放入仓库后重新启动 Codex。

在当前 Codex 中输入：

```text
/exit
```

然后：

```bash
cd ~/src/parade
codex
```

确认界面中的目录是：

```text
~/src/parade
```

模型保持：

```text
gpt-5.6-sol xhigh
```

可运行：

```text
/status
```

检查当前模型、目录和权限。

## 第四步：设置权限

在 Codex 中运行：

```text
/permissions
```

选择允许它在当前仓库内：

- 读取和修改文件；
- 运行构建与测试命令；
- 使用网络获取依赖；
- 执行 Git；
- 使用已经登录的 `gh` 推送分支并创建 Draft PR。

不要给予工作区之外无边界的系统权限，不要使用绕过全部安全限制的模式。

如果某个 GitHub 操作仍要求确认，睡前先允许该类操作，避免任务半夜停住。

## 第五步：粘贴启动提示词

在 Codex 输入框中输入 `@` 可以搜索仓库文件。将下面整段粘贴进去；也可以通过 `@` 补全各文件名。

```text
Read @AGENTS.md, @CODEX_GOAL.md, @TRAFFIC_ACCOUNTING_SPEC.md, and @UI_SPEC.md completely before making major design decisions.

Execute @CODEX_GOAL.md end to end. Begin by verifying the repository, current Git state, origin remote, default branch, and `gh auth status`. Preserve any pre-existing user changes. Create or switch to `codex/read-only-vps-observability`; never work directly on the default branch.

Create and continuously maintain PLANS.md. Audit the entire repository, verify the known defects, document the architecture and threat model, then implement the project rather than stopping after planning.

The product boundary is absolute: monitored VPSes are read-only observation targets. Never add arbitrary command execution, remote shell, process control, service control, package changes, firewall changes, user changes, file mutation, reboot, shutdown, reinstall, provider write actions, or automatic remediation. Hub-to-Agent requests may only select closed, typed, expiring read-only observation profiles.

Provider integrations are not part of this milestone. Implement the manual traffic model exactly as specified: I enter the provider's current-cycle used traffic, Parade stores it as an audited seed at an Agent checkpoint, adds subsequent locally observed traffic, and automatically rolls to zero at the configured monthly cycle boundary while preserving history and uncertainty.

Build a polished, fast, accessible dark/light UI according to UI_SPEC.md, including Fleet, server resources, privacy-preserving process inspection, network/listening ports, evidence-based security findings, events, and transparent traffic accounting. Optimize for large fleets and low bandwidth.

Use subagents for independent read-heavy architecture, security, test, and UI reviews when useful, but coordinate file ownership. Continue autonomously through implementation, tests, self-review, coherent commits, push to the existing origin, and draft PR creation. Do not merge.

Do not ask me broad design questions. Choose the safest reasonable default and document it. Ask only if safe progress is genuinely impossible. If an optional feature is blocked, continue all independent required work.

At the end, report the branch, commits, draft PR URL, checks run, bandwidth benchmark, synthetic fleet load result, screenshots, known limitations, and exact blockers. Do not claim a check passed unless it actually ran successfully.

Begin now.
```

## 第六步：让 Codex 持续运行

启动后观察前几分钟，确认它确实执行了：

```bash
git status --short --branch
git remote -v
gh auth status
```

并且建立了：

```text
PLANS.md
AUDIT.md
ARCHITECTURE.md
THREAT_MODEL.md
```

如果它只输出计划而没有修改代码，立即追加：

```text
Continue with implementation now. Do not wait for approval and do not stop at the planning phase. Work through the highest-priority complete vertical slice, run its tests, review the diff, commit it, then continue with the next phase.
```

## 睡前检查

确认：

- 服务器不会自动休眠；
- SSH 断开不会终止 Codex 所在的终端；
- 最好在 `tmux` 或 `screen` 中运行；
- 磁盘空间充足；
- Rust、Node.js、npm/pnpm、Git、`gh` 可用；
- `gh auth status` 成功；
- `origin` 指向正确仓库；
- 仓库不是公开状态，或者你明确接受当前公开性；
- 没有生产密钥放在仓库目录中。

推荐：

```bash
tmux new -s parade-codex
cd ~/src/parade
codex
```

断开 tmux：

```text
Ctrl-b d
```

重新连接：

```bash
tmux attach -t parade-codex
```

## 早晨检查

```bash
cd ~/src/parade
git status --short --branch
git log --oneline --decorate -20
gh pr status
gh pr view --web
```

重点查看：

- 是否仍在功能分支；
- 是否存在未提交大规模修改；
- 测试是否真实执行；
- PR 是否为 Draft；
- 是否意外加入远程控制能力；
- 流量基数、累加、重启恢复和周期切换测试；
- 进程参数或秘密是否泄露；
- UI 截图；
- 1000 Agent 压测；
- 月度低流量基准；
- `AUDIT.md` 和 `THREAT_MODEL.md`。

在一次性虚拟机或不重要的 VPS 上实际验证前，不要直接合并并部署到所有生产 VPS。
