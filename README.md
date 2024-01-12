## ncmdump-rs

A Rust implementation of [anonymous5l/ncmdump](https://github.com/anonymous5l/ncmdump)

[anonymous5l/ncmdump](https://github.com/anonymous5l/ncmdump) 的 Rust 实现。

## Usage

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