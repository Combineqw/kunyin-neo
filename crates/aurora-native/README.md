# aurora-native

坤音 neo 原生模块（R2 Rust 原生化）。影子阶段：只读实现，不接入主进程 / UI。

## 目录

| 文件 | 作用 |
| --- | --- |
| `src/lib.rs` | napi 导出层，对外只暴露返回 JSON 字符串的纯函数 |
| `src/error.rs` | 统一错误枚举 `AuroraError` + `Result` |
| `src/scan.rs` | 库扫描 / 元数据解析（R2-1 已完成） |
| `src/lyrics.rs` | 歌词解析 · LRC 行级（R2-2 已完成） |
| `src/settings_io.rs` | 设置读写（R2-3 占位） |

## 命令

```bash
npm run native:build     # 编译出 .node
npm run native:compare   # 只跑对答案（不重新编译）
npm run native:test      # 编译 + 对答案（一键，等价于双击 scripts/native-test.bat）
```

字段契约见仓库根 `PROJECT_STATUS.md`「scan 模块」表。改字段前先改表。

---

## 影子框架 · 注册新模块（供 R2-2 / R2-3）

只改 `scripts/shadow-modules.mjs`，往 `modules` 数组追加一项，框架本体不动：

```js
{
  id: 'lyrics',                                  // --module=lyrics 选它
  label: '歌词解析', unit: '行',                  // 报告里的显示名与量词
  keyField: 'path',                              // 两侧按此字段配对
  fields: ['time', 'text'],                      // 参与 diff 的字段
  describe: (it) => it.path,                     // 明细里怎么称呼一项
  async runNode(input) { /* 返回数组 */ },
  async runRust(input, native) { return JSON.parse(native.parseLyric(input)) },
  compare(field, a, b) { return a === b },       // 可选：自定义容差
}
```

Rust 侧对应在 `src/lib.rs` 加一个 `#[napi]` 函数返回 JSON 字符串即可。