# rsut-mcp-tools

一组用 Rust 编写的 MCP（Model Context Protocol）工具集，为 AI 提供本机能力扩展。

## 模块总览

| 模块 | 二进制名 | 功能概述 |
|---|---|---|
| [file-search-mcp](#file-search-mcp) | `file-search-mcp` | 文件内容搜索、文件名搜索、目录列举 |
| [command-bridge-mcp](#command-bridge-mcp) | `command-bridge-mcp` | 在本机执行 shell 命令，支持有状态 Session |
| [file-backup-mcp](#file-backup-mcp) | `file-backup-tool` | 目录结构导出、按规则备份、目录扁平化；可独立交互运行 |
| [win-service-control-mcp](#win-service-control-mcp) | `wsm` | Windows 服务/进程管理；可独立作为 CLI 使用 |

## 公共说明

### 前置要求

安装 [Rust 工具链](https://rustup.rs/)（stable 即可）。

### 编译所有模块

```bash
# debug 版（开发调试）
cargo build

# release 版（部署使用，体积更小、速度更快）
cargo build --release
```

编译产物统一位于：

- debug：`target/debug/<二进制名>.exe`
- release：`target/release/<二进制名>.exe`

---

## file-search-mcp

帮助 AI 在本地文件系统中快速定位内容和文件，解决 AI 自带搜索效率低、经常失败的问题。

### 工具

| 工具 | 说明 |
|---|---|
| `search_in_files` | 在目录下搜索包含指定字符串/正则的文件，返回文件路径和行号 |
| `search_files` | 在目录下按文件名搜索，支持精确匹配和 glob 模式 |
| `list_directory` | 列举指定目录下的文件和子目录，以树形结构返回 |

三个工具都支持可选参数 `ignore`（路径正则）。与启动参数 `--ignore` 合并后一起生效。

### 编译

```bash
cargo build --release -p file-search-mcp
```

产物：`target/release/file-search-mcp.exe`

### 启动参数

| 参数 | 说明 |
|---|---|
| `--ignore <正则>` | 全局忽略路径，匹配文件名或完整路径的文件/目录都会被跳过。支持 `\|` 多选分支，可多次使用 |

```bash
# 推荐写法：一条正则覆盖多个忽略规则
file-search-mcp.exe --ignore "node_modules|\.git|target|dist"
```

三个查询工具（`search_in_files`、`search_files`、`list_directory`）也都支持调用参数 `ignore`，格式与启动参数相同。启动 `--ignore` 与本次调用的 `ignore` **合并为并集**，两边命中的路径都会被跳过。例如启动时已忽略 `node_modules`，某次搜索再传 `"vendor|dist"`，则四者都会忽略。

### 配置

**Cursor**（`%APPDATA%\Cursor\User\globalStorage\rooveterinaryinc.roo-cline\settings\mcp_settings.json`）：

```json
{
  "mcpServers": {
    "file-search": {
      "command": "D:\\path\\to\\file-search-mcp.exe",
      "args": ["--ignore", "node_modules|\\.git|target|dist"]
    }
  }
}
```

**Claude Desktop**（`%APPDATA%\Claude\claude_desktop_config.json`）：

```json
{
  "mcpServers": {
    "file-search": {
      "command": "D:\\path\\to\\file-search-mcp.exe",
      "args": ["--ignore", "node_modules|\\.git|target|dist"]
    }
  }
}
```

### 特性说明

- 自动跳过二进制文件（exe、dll、图片、音视频等）
- 自动处理 GBK / UTF-8 编码
- 基于 `rayon` 多线程并行搜索，大型项目速度快
- 单文件部署，无需运行时
- 忽略规则：启动 `--ignore` + 调用参数 `ignore` 取并集；命中的目录不会再进入子目录

---

## command-bridge-mcp

让 AI 能在本机执行 shell 命令。提供两种模式：无状态单次执行，以及有状态 Session 模式（支持跨调用保持工作目录）。

### 工具

| 工具 | 模式 | 说明 |
|---|---|---|
| `run_command` | 无状态 | 执行单条命令，每次调用均为独立新进程，`cd` 不会跨调用保留 |
| `open_session` | Session | 创建有状态 shell 会话，返回 `session_id` |
| `run_in_session` | Session | 在已有 Session 中执行命令，`cd` 跳转在 Session 内持久保留 |
| `close_session` | Session | 关闭 Session 并释放资源，使用完毕后必须调用 |

**Session 使用流程：**

```
open_session → run_in_session（可多次）→ close_session
```

### 限制

- 禁止执行删除命令（`del`、`rd`、`rm`、`Remove-Item` 等）
- 禁止执行交互式程序（`vim`、`ssh` 等）
- Session 模式下仅工作目录（`cd`）跨调用保留，环境变量不保留

### 编译

```bash
cargo build --release -p command-bridge-mcp
```

产物：`target/release/command-bridge-mcp.exe`

### 启动参数

无额外启动参数，直接运行即可。

### 配置

**Cursor**（`%APPDATA%\Cursor\User\globalStorage\rooveterinaryinc.roo-cline\settings\mcp_settings.json`）：

```json
{
  "mcpServers": {
    "command-bridge": {
      "command": "D:\\path\\to\\command-bridge-mcp.exe"
    }
  }
}
```

**Claude Desktop**（`%APPDATA%\Claude\claude_desktop_config.json`）：

```json
{
  "mcpServers": {
    "command-bridge": {
      "command": "D:\\path\\to\\command-bridge-mcp.exe"
    }
  }
}
```

---

## file-backup-mcp

扫描目录、按规则并发备份文件，以及扁平化目录结构。同一个可执行文件同时支持交互式 CLI 和 MCP 两种模式。

### 运行模式

| 模式 | 启动方式 | 说明 |
|---|---|---|
| 交互式 CLI | 直接运行，不加参数 | 终端菜单引导完成备份 / 结构导出 / 扁平化 |
| MCP Server | `file-backup-tool.exe mcp` 或 `--mcp` | 通过 stdio 对接 Cursor / Claude Desktop |

### 工具

| 工具 | 说明 |
|---|---|
| `generate_directory_structure` | 扫描目录，生成 JSON 结构文件和 TXT 树状预览 |
| `backup_files` | 按正则匹配路径并发备份文件，可选择忽略空文件夹 |
| `flatten_directory` | 将子目录中的文件全部提到根目录（不可逆） |

### 编译

```bash
cargo build --release -p file-backup-mcp
```

产物：`target/release/file-backup-tool.exe`

### 配置

**Cursor：**

```json
{
  "mcpServers": {
    "file-backup": {
      "command": "D:\\path\\to\\file-backup-tool.exe",
      "args": ["mcp"]
    }
  }
}
```

**独立运行：**

```bash
# 交互式菜单（双击 exe 或直接运行）
file-backup-tool.exe

# MCP 模式
file-backup-tool.exe mcp
```

### 特性说明

- 基于 `rayon` 多线程并行复制
- CLI 模式带进度条；MCP 模式静默执行，只返回结果摘要
- Windows 下会把图标嵌入 exe，便于独立分发

---

## win-service-control-mcp

Windows 服务和进程管理工具。同一个 `wsm.exe` 既是命令行工具，也可以作为 MCP Server。

### 运行模式

| 模式 | 启动方式 | 说明 |
|---|---|---|
| CLI | `wsm <子命令>` | 独立命令行，例如 `wsm list`、`wsm ps chrome` |
| MCP Server | `wsm mcp` | 通过 stdio 对接 Cursor / Claude Desktop |

操作服务（启动/停止/改启动类型）通常需要**管理员权限**。

### 工具

| 工具 | 说明 |
|---|---|
| `manage_services` | 查询、启动、停止 Windows 服务，可批量操作并修改启动类型 |
| `manage_processes` | 按进程名或 PID 查询、终止进程 |

### CLI 子命令

```bash
wsm list [关键字]          # 列出运行中的服务
wsm list-all [关键字]      # 列出全部服务
wsm open <服务名...> [-p] [-m]   # 启动服务；-p 设为自动，-m 设为手动
wsm stop <服务名...> [-p] [-m]   # 停止服务；-p 设为禁用，-m 设为手动
wsm ps [关键字]            # 列出进程
wsm kill <进程名或PID...>  # 终止进程
wsm mcp                    # 进入 MCP 模式
```

服务名和进程名都支持逗号分隔批量操作。核心系统服务/进程有保护名单，禁止停止或终止。

### 编译

```bash
cargo build --release -p win-service-control-mcp
```

产物：`target/release/wsm.exe`

### 配置

**Cursor：**

```json
{
  "mcpServers": {
    "win-service-control": {
      "command": "D:\\path\\to\\wsm.exe",
      "args": ["mcp"]
    }
  }
}
```

**独立运行示例：**

```bash
wsm list spooler
wsm open Spooler
wsm ps chrome
wsm kill notepad
```
