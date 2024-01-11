use structopt::StructOpt;
use libncm::{DumpTarget, dump};

#[derive(StructOpt, Debug)]
#[structopt(
    name = "ncmdump",
    about = "A tool to dump .ncm files 一个用于解密 .ncm 文件的工具"
)]
struct Opt {
    /// The .ncm file to dump 要解密的 .ncm 文件
    #[structopt(parse(from_os_str), help = "The .ncm file to dump 要解密的 .ncm 文件")]
    file: std::path::PathBuf,

    /// The output directory 输出目录
    #[structopt(
        parse(from_os_str),
        short = "o",
        long = "output",
        default_value = ".",
        help = "The output directory 输出目录"
    )]
    output_dir: std::path::PathBuf,

    /// Dump target 解密目标
    #[structopt(
        short = "t",
        long = "target",
        default_value = "all",
        help = "Dump target 解密目标 (all, audio, image)"
    )]
    target: String,
}

fn main() {
    let opt = Opt::from_args();
    let file_path = opt.file.to_str().unwrap();
    let output_dir = opt.output_dir.to_str().unwrap();
    let target = match opt.target.as_str() {
        "all" => DumpTarget::ALL,
        "audio" => DumpTarget::AUDIO,
        "image" => DumpTarget::IMAGE,
        _ => {
            println!("Invalid dump target");
            return;
        }
    };
    if let Err(e) = dump(file_path, output_dir, target) {
        println!("Error: {}", e);
    }
}
