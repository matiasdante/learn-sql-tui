<div align="center">

[English](../../README.md) · [Español](README_es.md) · [Português (Brasil)](README_pt-BR.md) · **简体中文**

# learn-sql-tui

### 在终端里用真实的 SQL 练习，学会 SQL。

**11 个级别、61 节课、90 道实操练习，支持英语、西班牙语、葡萄牙语和中文。每条语句都运行在程序内置的真实 SQLite 上，处于隔离的沙盒中。**

</div>

> [!NOTE]
> 无需安装任何东西：SQLite 已编译进可执行文件，程序离线也能使用。它只会在自己的数据文件夹内读写。

---

每道练习给你一个小场景和一个控制台。怎么解决都行；按 `F5` 时检查的是**数据库的状态**（表、行、索引、pragma），而不是你输入了哪些语句，所以任何正确的做法都算数。只用 SELECT 的练习会把你最后一次的查询结果与参考查询的结果比较，只有题目要求 `ORDER BY` 时顺序才有意义。如果检查失败，应用会说明缺少什么，并把沙盒恢复到练习开始时的状态。

## 快速开始

### 环境要求

- 运行时不需要任何东西：Windows 或 Linux 的单个可执行文件，无需网络，也无需安装 SQLite。
- 从源码构建：Rust stable（2024 edition，1.85 或更新）和 C 编译器（SQLite 从源码编译；Windows 需要 MSVC 构建工具，Linux 需要 `gcc` 或 `cc`，静态构建还需要 `musl-tools`）。

### 安装

从 [Releases](https://github.com/matiasdante/learn-sql-tui/releases) 页面下载适合你系统的可执行文件：

| 系统 | 文件 |
| --- | --- |
| Windows x86_64 | `learn-sql-tui-windows-x86_64.exe` |
| Linux x86_64（静态） | `learn-sql-tui-linux-x86_64` |

也可以用 cargo 构建（这也是在 macOS 上使用的方式）：

```sh
cargo install --git https://github.com/matiasdante/learn-sql-tui
```

### 运行

```sh
learn-sql-tui
learn-sql-tui --version
```

首次启动时选择语言，并可参加 10 道题的分级测试（第 1 到 5 级各两道）。全对则从第 6 级开始；跳过则从第 1 级开始。之前的级别始终可用。

## 工作原理

- **控制台。** SQL 语句以 `;` 结尾，可以写成多行（提示符变为 `...>`；事务打开时为 `sql*>`）。点命令与 `sqlite3` 命令行一致：`.tables`、`.schema`、`.indexes`、`.mode box|csv|line`、`.headers on|off`、`.read 文件`、`.import [--skip N] 文件 表`、`.help`、`.clear`；`edit 文件` 打开内置编辑器。结果以表格显示（最多 200 行）。
- **沙盒。** 练习数据库是沙盒文件夹中的 `practice.db`；快照和恢复会在关闭连接的情况下复制该文件。自由模式有自己的 `free.db`，会在多次使用之间保留。
- **隔离。** authorizer 会拒绝沙盒文件夹之外的 `ATTACH` 以及 `load_extension`；`VACUUM INTO`、`.read`、`.import` 和 `edit` 的路径都在该文件夹内解析；并启用了 `SQLITE_DBCONFIG_DEFENSIVE`。有测试验证无法在沙盒之外写入。
- **Ctrl+C。** 语句在后台线程中运行，`Ctrl+C` 可以中断它们（例如无限的递归 CTE）。
- **检查。** `table_exists`、`column`、`foreign_key`、`index_exists`、`view_exists`、`trigger_exists`、`row_count`、`query_equals`、`pragma_equals`、`last_result_same_as`、`autocommit`……它们在当前会话上运行，所以 `PRAGMA foreign_keys` 和未提交的事务都会被计入。
- **外键** 默认关闭（OFF），与 `sqlite3` 命令行一致，因为第 7 级会教你如何开启它们。

## 级别

| # | 级别 | 课程 | 练习 |
| --- | --- | --- | --- |
| 1 | 基础查询：SELECT、WHERE、ORDER BY、LIMIT、DISTINCT、NULL、LIKE、IN/BETWEEN、CASE | 9 | 16 |
| 2 | 函数与聚合：文本/数值/日期、COUNT…MAX、GROUP BY、HAVING、COALESCE、CAST | 7 | 13 |
| 3 | 连接查询：INNER、LEFT、RIGHT/FULL、CROSS、自连接、USING | 6 | 9 |
| 4 | 子查询与集合运算：IN/EXISTS、相关子查询、UNION/INTERSECT/EXCEPT、CTE、递归 CTE | 5 | 10 |
| 5 | 修改数据：INSERT、UPDATE、DELETE、UPSERT、RETURNING | 5 | 7 |
| 6 | 表结构：CREATE/ALTER/DROP、类型与 STRICT、约束、外键、生成列 | 7 | 9 |
| 7 | 事务与完整性：BEGIN/COMMIT/ROLLBACK、SAVEPOINT、foreign_keys、foreign_key_check | 4 | 5 |
| 8 | 视图、索引与性能：VIEW、部分索引与表达式索引、EXPLAIN QUERY PLAN、ANALYZE | 5 | 8 |
| 9 | 高级 SQL：窗口函数、触发器、JSON、FTS5 | 4 | 7 |
| 10 | 数据库管理：ATTACH、VACUUM、integrity_check、导入 CSV、user_version | 5 | 6 |
| 11 | 超越 SQLite（仅讲解）：方言、GRANT/REVOKE、隔离级别、SQL 注入 | 4 | 0 |

完成最后一道实操练习后，会出现 **接下来做什么？** 界面（并保留在地图中）。它推荐 [lazysql](https://github.com/jorgerojas26/lazysql)（一个数据库终端界面），说明如何安装，并显示你的自由模式数据库路径，方便在那里打开。lazysql 只是推荐，本程序从不启动它。

## 语言

英语、西班牙语（`es`）、葡萄牙语（`pt`）和中文（`zh`）。在菜单中按 `L` 切换，选择会被保存。所有文本都在 `content/i18n/<语言>/` 中，任何语言缺少文本时测试都会失败。

## 操作

| 按键 | 作用 |
| --- | --- |
| `↑` `↓` · `Enter` · `Esc` | 移动、打开、返回 |
| `L` | 切换语言（菜单中） |
| `F1` | 显示下一条提示（−25 分） |
| `F5` | 检查练习（每次检查失败扣 10 分） |
| `F6` | 下一道练习 |
| `Ctrl+R` | 重新开始练习（自由模式中：删除自由模式数据库） |
| `Ctrl+C` | 停止正在运行的语句，或清空当前行 |
| `Ctrl+S` · `Esc` | 编辑器中：保存并关闭 · 取消 |
| `Ctrl+Q` | 退出 |

每道练习 100 分，最低 25 分。另有**自由模式**和 29 个**成就**。

## 数据存放位置

所有数据都在用户数据目录下的 `learn-sql-tui` 文件夹中（Linux 为 `~/.local/share`，Windows 为 `%APPDATA%`）：

| 路径 | 内容 |
| --- | --- |
| `progress.json` | 语言、已完成的练习、分数、成就 |
| `sandbox/practice.db` | 当前练习的数据库 |
| `sandbox/work/` | 练习用的文件（`.read`、`.import`、`ATTACH`） |
| `sandbox/free/free.db` | 你的自由模式数据库（可用 lazysql 打开） |

删除 `progress.json` 即可从头开始。

## 常见问题

<details>
<summary><strong>它会改动自己文件夹之外的文件吗？</strong></summary>

不会。路径都在沙盒内解析，`..` 会被拒绝，指向沙盒之外的 `ATTACH` 和 `VACUUM INTO` 会失败，也无法加载扩展。

</details>

<details>
<summary><strong>必须严格输入课程建议的语句吗？</strong></summary>

不必。检查看的是最终状态，所以任何正确的方式都可以。有些主题不会留下状态（`.tables`、`EXPLAIN`），这些情况下应用会查看你输入的内容。

</details>

<details>
<summary><strong>支持 PostgreSQL 或 MySQL 吗？</strong></summary>

不支持，只支持 SQLite。第 11 级会讲解与其他数据库引擎的差异、权限、隔离级别和 SQL 注入，没有练习。

</details>

## 开发

```sh
cargo build
cargo test            # 引擎、隔离、Ctrl+C、每道练习对照其参考解答
cargo clippy --all-targets -- -D warnings
cargo fmt --check
```

课程是 `content/levelNN/` 中的 TOML 文件，共享的数据脚本在 `content/seeds/`，文本在 `content/i18n/<语言>/`。内容测试会启动每道练习，要求未改动的状态通不过检查，而参考解答能通过。

## 许可证

MIT 许可证。
