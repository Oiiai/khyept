# kyptc

Khyept 的最小解释器原型，使用 Rust 编写。当前支持注释、`let`/`var` 变量、结构体（含 `} 实例名;` 声明形式）、函数和 `return`、局部/全局变量、基础类型与算术/比较表达式、列表/字典字面量和下标访问、类型转换与 `type`/`size` 内置函数、`if`/`elif`/`else` 条件语句和条件表达式、`switch`/`case`/`default`、`for`/`while`/`do while` 循环、`break`、`print`/`println`、`readln` 以及 `f"...{表达式}..."` 字符串插值。

## 运行

需要 Rust stable 工具链：

```sh
cargo run -- examples/basics.khyept
```

构建独立可执行文件：

```sh
cargo build --release
```

当前入口按源文件顺序执行顶层语句；函数定义可放在调用之前或之后。支持类型标注 `int`、`float`、`string`、`bool`、`void`，省略标注时按值自动推断。`readln()` 或 `readln("提示")` 返回一行字符串。单引号和双引号字符串（包括 f-string 文本）支持 `\\n`、`\\t`、`\\r`、`\\b`、`\\f`、`\\v`、`\\a`、`\\0`、`\\ooo`、`\\xhh`、`\\uhhhh` 与 `\\Uhhhhhhhh` 转义。

`print(value)` 输出内容但不添加换行符；`println(value)` 输出内容后添加换行符。`int`、`float`、`string`、`bool` 可将标量值转换为目标类型；`type(value)` 返回类型名，`size(list_or_dict)` 返回容器长度。列表下标从 0 开始，越界下标和字典缺失键返回 `0`。

结构体使用 `struct Name { let type fixed; var type mutable; } object;` 声明，实例名可有可无，也可一次声明多个（`} a, b;`），或带初始化值（`} a{1, "x"};`）。带实例名时对象按 `type` 零值初始化，且作为全局变量存在；`let` 成员必须在对象创建时提供，之后不可修改；`var` 成员可以通过 `object.member = value` 修改。等价的旧写法 `struct Name { ... };` 配合 `var Name object{...};` 仍然可用。成员可以是标量、列表或字典，也支持链式成员访问和下标访问。结构体不能直接转换为其他类型。

未提供初始化值时的成员默认值：`int` 为 `0`，`float` 为 `0.0`，`string` 为空串，`bool` 为 `false`，其他（含未标注、嵌套结构体）为 `null`。

下面两个程序等价：

```khyept
struct student {
    var int id;
    var int age;
    var string name;
} s;

fn int main() {
    s.id = int(readln());
    s.age = int(readln());
    s.name = string(readln());
    println(s.id);
    println(s.age);
    println(s.name);
    return(0);
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

## 暂不支持

逻辑运算符、隐式数值类型转换，以及完整 Python 风格格式表达式暂不支持。`if` 和循环条件必须是 `bool` 值；条件表达式分支取代码块最后一个表达式的值。`switch` 支持多值 `case(a, b)`、闭区间 `a...b`、左开右闭区间 `a<..b`、左闭右开区间 `a..<b` 和 fall-through；`break` 可停止当前 `switch` 或循环。`global name;` 需要在对应函数作用域中先声明，再通过变量声明创建全局变量，例如 `global count; var int count = 1;`。
