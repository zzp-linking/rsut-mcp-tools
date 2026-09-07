# rsut-mcps

一组用 Rust 编写的 MCP（Model Context Protocol）工具集，为 AI 提供本机能力扩展。

## 模块总览

| 模块 | 二进制名 | 功能概述 |
|---|---|---|
| [file-search-mcp](#file-search-mcp) | `file-search-mcp` | 文件内容搜索、文件名搜索、目录列举 |
| [command-bridge-mcp](#command-bridge-mcp) | `command-bridge-mcp` | 在本机执行 shell 命令，支持有状态 Session |

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

### 编译

```bash
cargo build --release -p file-search-mcp
```

产物：`target/release/file-search-mcp.exe`

### 启动参数

| 参数 | 说明 |
|---|---|
| `--ignore <正则>` | 忽略路径匹配该正则的文件或目录，支持 `\|` 多选分支，可多次使用 |

```bash
# 推荐写法：一条正则覆盖多个忽略规则
file-search-mcp.exe --ignore "node_modules|\.git|target|dist"
```

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
