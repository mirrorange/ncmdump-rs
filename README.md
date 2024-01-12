## ncmdump-rs

[anonymous5l/ncmdump](https://github.com/anonymous5l/ncmdump) 的 Rust 实现。

A Rust implementation of [anonymous5l/ncmdump](https://github.com/anonymous5l/ncmdump)

## 使用方法 Usage

你可以在 [Releases](https://github.com/mirrorange/ncmdump-rs/releases) 页面下载预编译的二进制文件。其中 `ncmdumpcli` 是命令行版本，`liblibncm.so` 或 `libncm.dll` 是动态链接库版本，`liblibncm.rlib` 是 Rust 静态链接库版本。

You can download precompiled binaries from [Releases](https://github.com/mirrorange/ncmdump-rs/releases) page. `ncmdumpcli` is the command line version, `liblibncm.so` or `libncm.dll` is the dynamic library version, `liblibncm.rlib` is the Rust static library version.

### 命令行版本 Command Line
```
USAGE:
    ncmdumpcli [OPTIONS] <file>

FLAGS:
    -h, --help       Prints help information
    -V, --version    Prints version information

OPTIONS:
    -o, --output <output-dir>    The output directory 输出目录 [default: .]
    -t, --target <target>        Dump target 解密目标 (all, audio, image) [default: all]

ARGS:
    <file>    The .ncm file to dump 要解密的 .ncm 文件
```

### 动态链接库版本 Dynamic Library

C# Example: (Windows)

```csharp
using System;
using System.IO;
using System.Runtime.InteropServices;

namespace ncmdump
{
    class Program
    {
        [DllImport("libncm", CallingConvention = CallingConvention.Cdecl)]
        static extern int dump_wrapper(string file_path, string output_dir, int target);

        static void Main(string[] args)
        {
            if (args.Length < 1)
            {
                Console.WriteLine("Usage: ncmdump <input> [output] [target]");
                return;
            }

            string input = args[0];
            string output = args.Length > 1 ? args[1] : ".";
            string target = args.Length > 2 ? args[2] : "all";
            int target_code = 0;

            switch (target)
            {
                case "all":
                    target_code = 0;
                    break;
                case "audio":
                    target_code = 1;
                    break;
                case "image":
                    target_code = 2;
                    break;
                default:
                    Console.WriteLine("Invalid target.");
                    return;
            }
            if (!File.Exists(input))
            {
                Console.WriteLine("Input file not found.");
                return;
            }
            if (!Directory.Exists(output))
            {
                Console.WriteLine("Output directory not found.");
                return;
            }

            int ret = dump_wrapper(input, output, target_code);

            if (ret == 0)
            {
                Console.WriteLine("Done.");
            }
            else
            {
                Console.WriteLine("Error.");
            }
        }
    }
}
```

Python Example: (Linux)

```python
import ctypes
import os

lib = ctypes.cdll.LoadLibrary("libncm.so")

def dump(file_path, output_dir, target):
    return lib.dump_wrapper(file_path.encode("utf-8"), output_dir.encode("utf-8"), target)

if __name__ == "__main__":
    file_path = input("Input file: ")
    output_dir = input("Output directory: ")
    target = input("Target (all, audio, image): ")
    target_code = 0

    if target == "all":
        target_code = 0
    elif target == "audio":
        target_code = 1
    elif target == "image":
        target_code = 2
    else:
        print("Invalid target.")
        exit(1)

    if not os.path.exists(file_path):
        print("Input file not found.")
        exit(1)
    if not os.path.exists(output_dir):
        print("Output directory not found.")
        exit(1)

    ret = dump(file_path, output_dir, target_code)

    if ret == 0:
        print("Done.")
    else:
        print("Error.")
```

## 从源代码构建 Build

```
cargo build --release
```