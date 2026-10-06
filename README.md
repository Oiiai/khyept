# kyptc

Khyept 的最小解释器原型，使用 Rust 编写。当前支持注释、`let`/`var` 变量、结构体与表达式式构造、`fn` 函数与 `return`、局部/全局变量、基础类型与算术/比较/逻辑/位运算表达式、列表/字典/字符串的下标读写与字符串切片、`len()` 与 `.size()`、文件 IO（`open` / `read` / `write` / `writeln`）、类型转换与 `type` 内置函数、`if`/`elif`/`else` 条件语句和条件表达式、`switch`/`case`/`default`、`for`/`while`/`do while` 循环、`break`、`print`/`println`、`readln` 以及 `f"...{表达式}..."` 字符串插值。

## 注释

行注释用**三个**斜杠 `///`，块注释用 `/* ... */`：

```khyept
/// 这是行注释
var a := 1;   /// 也可以写在代码后面

/*
 * 这是块注释，可以跨多行
 */
var b := 2;
```

行注释必须是 `///`。两个斜杠 `//` 是整除运算符，不作注释：

```khyept
var q := 7 // 2;    /// q == 3，这是整除
```

这样设计让 `//` 专属于整除，不需要靠上下文猜测消歧，词法层也不会再把中文注释误判成运算符。

## 语法速览

类型标注写在名字**后面**，用 `:` 分隔；函数返回类型用 `->` 标注，不写时默认为 `void`。

```khyept
var n : int = 5;          /// 变量声明：类型在后
let s : string = "A";     /// 常量声明
var x := 5;               /// 由解释器自动判断类型（类似 GDScript 的 :=）

fn add(a : int, b : int) -> int {   /// 参数与返回值都写在后面
    return(a + b);
}

fn greet() {              /// 不写返回类型，默认 void
    println("hello");
}
```

`var x := 5` 会在运行期推断出类型并**锁定**该变量，之后给它赋不同类型的值会报错：

```khyept
var x := 5;
x = 10;         /// 正常
x = "字符串";   /// 错误：变量 `x`：类型 `int` 与值 `字符串` 不匹配
```

函数声明了返回类型但没有 `return` 会报错；返回值的类型也会被检查：

```khyept
fn bad() -> int {
    return("不是 int");   /// 错误：函数 `bad` 返回值：类型 `int` 与值 `不是 int` 不匹配
}
```

支持类型标注 `int`、`float`、`string`、`bool`、`void`，省略标注时按值自动推断。`readln()` 或 `readln("提示")` 返回一行字符串。单引号和双引号字符串（包括 f-string 文本）支持 `\n`、`\t`、`\r`、`\b`、`\f`、`\v`、`\a`、`\0`、`\ooo`、`\xhh`、`\uhhhh` 与 `\Uhhhhhhhh` 转义。

> 旧的前置类型写法（`var int n = 5`、`fn int add(int a, int b)`、`struct { var int x; }`）仍然兼容，但推荐使用新写法。

## 运行

需要 Rust stable 工具链。在 Git Bash 下请使用附带的构建脚本（它会处理 MSVC 链接器与 SDK 路径）：

```sh
./build.sh run -- examples/basics.khyept
```

在其他环境下可直接用 cargo：

```sh
cargo run -- examples/basics.khyept
```

构建独立可执行文件：

```sh
./build.sh build --release
# 产物：target/release/kyptc.exe
```

当前入口按源文件顺序执行顶层语句；函数定义可放在调用之前或之后。支持类型标注 `int`、`float`、`string`、`bool`、`void`，省略标注时按值自动推断。`readln()` 或 `readln("提示")` 返回一行字符串。单引号和双引号字符串（包括 f-string 文本）支持 `\\n`、`\\t`、`\\r`、`\\b`、`\\f`、`\\v`、`\\a`、`\\0`、`\\ooo`、`\\xhh`、`\\uhhhh` 与 `\\Uhhhhhhhh` 转义。

`print(value)` 输出内容但不添加换行符；`println(value)` 输出内容后添加换行符。`int`、`float`、`string`、`bool` 可将标量值转换为目标类型；`type(value)` 返回类型名，`size(list_or_dict)` 返回容器长度。列表下标从 0 开始，越界下标和字典缺失键返回 `0`。

结构体使用 `struct Name { let name : type; var name : type; } object;` 声明，实例名可有可无，也可一次声明多个（`} a, b;`），或带初始化值（`} a{1, "x"};`）。带实例名时对象按 `type` 零值初始化，且作为全局变量存在；`let` 成员必须在对象创建时提供，之后不可修改；`var` 成员可以通过 `object.member = value` 修改。等价的旧写法 `struct Name { ... };` 配合 `var Name object{...};` 仍然可用。成员可以是标量、列表或字典，也支持链式成员访问和下标访问。结构体不能直接转换为其他类型。

未提供初始化值时的成员默认值：`int` 为 `0`，`float` 为 `0.0`，`string` 为空串，`bool` 为 `false`，其他（含未标注、嵌套结构体）为 `null`。

下面两个程序等价：

```khyept
struct student {
    var id : int;
    var age : int;
    var name : string;
} s;

fn main() {
    s.id = int(readln());
    s.age = int(readln());
    s.name = string(readln());
    println(s.id);
    println(s.age);
    println(s.name);
}

main();
```

```cpp
#include <iostream>
#include <string>
using namespace std;

struct student {
    int id;
    int age;
    string name;
} s;

int main() {
    string ln;
    getline(cin, ln); s.id = stoi(ln);
    getline(cin, ln); s.age = stoi(ln);
    getline(cin, s.name);
    cout << s.id   << endl;
    cout << s.age  << endl;
    cout << s.name << endl;
    return 0;
}
```

## 运算符

| 运算符 | 含义 | 示例 | 结果 |
|---|---|---|---|
| `//` | 整除 | `7 // 2` | `3` |
| `/` | 除法 | `7 / 2` | `3` |
| `%` | 取余 | `7 % 2` | `1` |
| `**` | 幂（右结合） | `2 ** 3 ** 2` | `512` |
| `&` | 按位与 | `5 & 3` | `1` |
| `\|` | 按位或 | `5 \| 3` | `7` |
| `^` | 按位异或 | `5 ^ 3` | `6` |
| `~` | 按位取反 | `~5` | `-6` |
| `<<` | 左移 | `5 << 1` | `10` |
| `>>` | 右移 | `5 >> 1` | `2` |
| `&&` | 逻辑与（短路） | `a == 5 && b == 7` | |
| `\|\|` | 逻辑或（短路） | `a == 5 \|\| b == 7` | |
| `!` | 逻辑非 | `!(a == 5)` | |

位运算只作用于 `int`；`&&`、`||`、`!` 只作用于 `bool`。`//` 是整除运算符（注释请用 `///`），`**` 右结合，即 `2 ** 3 ** 2 == 2 ** (3 ** 2)`。

## 列表与字典的下标赋值

可以直接写入列表或字典的某个位置：

```khyept
var l : list = [0, 1, 2, 3];
l[1] = 20;              /// 覆盖已有元素
l[size(l)] = 99;        /// 下标等于长度时追加
println(l);             /// [0, 20, 2, 3, 99]

var g : list = [[0, 0], [0, 0]];
g[0][1] = 5;            /// 嵌套赋值也可以

var d : dict = {"a": 1};
d["a"] = 10;             /// 覆盖
d["b"] = 20;             /// 新增键
```

越界写入会报错（只能在下标等于长度时追加），避免误写。`list` 和 `dict` 现在也可以作为类型标注使用。

## 简写语句体

`if` / `elif` / `else` / `while` / `for` / `case` 后面不强求花括号，只跟一条语句也可以：

```khyept
if (a == 5) println("a5");
elif (a == 6) println("a6");
else println("other");

while (i < 10) i++;
```

## 示例

- `examples/bfs_grid.khyept` —— BFS 网格最短路径（用列表下标赋值实现队列与访问标记）
- `examples/dp_knapsack.khyept` —— 0-1 背包动态规划
- `examples/n_queens_dfs.khyept` —— N 皇后，DFS + 回溯

## 字符串下标与切片

字符串支持下标读写与切片：

```khyept
var s : string = "Hello, World!";
s[0] = "h";              /// 写入单字符，必须恰好 1 个字符
println(s + " " + s[0]);
println(s[1:]);          /// ello, World!
println(s[:5]);          /// hello
println(s[0:5]);         /// hello
println(s[:]);           /// 整个字符串
```

下标与长度按**字符**计算（不是字节），所以中文也能正确处理。越界会报错。

## len() 与 .size()

两者等价，对字符串、列表、字典都可用：

```khyept
var s := "Hello, World!";    /// 长度 13
println(len(s));
println(s.size());

var l : list = [1, 2, 3];
var d : dict = {"a": 1, "b": 2};
println(len(l));             /// 3
println(l.size());           /// 3
println(len(d));             /// 2
println(d.size());           /// 2
```

## 文件 IO

`open()` 返回一个文件句柄（类型标注写 `string` 即可），通过 `read` / `write` / `writeln` / `close` 操作：

```khyept
/// 写入（覆盖）
var f : string = open("222.txt", "w", encoding="utf-8");
f.write("Hello, World!");
f.close();

/// 读取
var g : string = open("222.txt", "r", encoding="utf-8");
println(g.read());
g.close();

/// 写入（追加）
var h : string = open("222.txt", "a");
h.write("\n追加一行~");
h.close();

/// 一次写入多行
var lines := ["第一行\n", "第二行\n"];
var f2 : string = open("444.txt", "w", encoding="utf-8");
f2.writeln(lines);
f2.close();

/// 写入二进制，列表元素视为字节
var data : list = [0x00, 0x01, 0x02, 0x03];
var f3 : string = open("333.bin", "wb");
f3.write(data);
f3.close();
```

`encoding` 是命名参数，默认为 `utf-8`，可选 `ascii`、`gbk`、`latin1`。

### 文件模式

| 模式 | 读取 | 写入 | 打开时清空 |
|---|---|---|---|
| `r` | yes | no | no |
| `w` | no | yes | yes |
| `a` | no | yes | no |
| `r+` | yes | yes | no |

`r+` 从文件开头覆盖写入，行为类似 Python：

```khyept
var f : string = open("test.txt", "w");
f.write("Hello World");
f.close();

var g : string = open("test.txt", "r+");
g.write("ABC");           /// 内容变成 "ABClo World"
g.close();
```

`r+` 模式下多次 `write` 会顺序推进写入位置，不会互相覆盖。二进制变体有 `rb`、`wb`、`ab`、`rb+`。

## 结构体

结构体可以用表达式形式构造，位置任意：

```khyept
struct Point { var x : int; var y : int; }

var p = Point{1, 2};        /// 表达式形式
var q : Point = Point{3, 4};
```

也保留了声明式写法：`} p{1, 2};`。两种形式等价。

## 已知限制

隐式数值类型转换，以及完整 Python 风格格式表达式暂不支持。`if` 和循环条件必须是 `bool` 值；条件表达式分支取代码块最后一个表达式的值。`switch` 支持多值 `case(a, b)`、闭区间 `a...b`、左开右闭区间 `a<..b`、左闭右开区间 `a..<b` 和 fall-through；`break` 可停止当前 `switch` 或循环。`global name;` 需要在对应函数作用域中先声明，再通过变量声明创建全局变量，例如 `global count; var count : int = 1;`。

以下能力尚未实现：

- **模块系统**：目前只有单文件，无法 `import` 或拆分模块
- **异常机制**：运行出错会直接终止程序，无法捕获与恢复
- **一等函数**：函数不能当作值传递或作为回调参数
- **递归深度上限 256**，深层嵌套表达式可能超限
- `print` 只接受值，不支持 f-string；要输出请用 `println`
- `gbk` 编码为简化实现，非 ASCII 字符按 UTF-8 字节处理
- f-string 内不能嵌套同类引号，`f"...{d["k"]}..."` 报错，需外双引号内单引号
