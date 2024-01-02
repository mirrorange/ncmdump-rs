use aes::Aes128;
use base64::decode;
use block_modes::block_padding::Pkcs7;
use block_modes::{BlockMode, Ecb};
use byteorder::{LittleEndian, ReadBytesExt};
use hex::FromHex;
use serde_json::Value;
use std::fs::File;
use std::io::{self, prelude::*, Seek, SeekFrom};
use std::path::Path;
use structopt::StructOpt;

#[derive(PartialEq, Eq)]
enum DumpTarget {
    ALL,
    AUDIO,
    IMAGE,
}

struct NcmDump {
    core_key: Vec<u8>,
    meta_key: Vec<u8>,
}

impl NcmDump {
    fn new() -> NcmDump {
        NcmDump {
            core_key: Vec::from_hex("687A4852416D736F356B496E62617857").unwrap(),
            meta_key: Vec::from_hex("2331346C6A6B5F215C5D2630553C2728").unwrap(),
        }
    }

    fn open_ncm_file(&self, file_path: &str) -> io::Result<File> {
        let mut f = File::open(file_path)?;

        // Check the file header 检查文件头
        let mut header = [0; 8];
        f.read_exact(&mut header)?;
        if &header != b"CTENFDAM" {
            return Err(io::Error::new(io::ErrorKind::Other, "Invalid file header"));
        }

        Ok(f)
    }

    fn dump(&self, file_path: &str, output_dir: &str, target: DumpTarget) -> io::Result<()> {
        // Open the .ncm file 打开 .ncm 文件
        let mut f = self.open_ncm_file(file_path)?;

        // Decrypt the key 解密 key
        let key_box = self.decrypt_key(&mut f)?;

        // Decrypt the metadata 解密元数据
        let meta_data = self.decrypt_meta_data(&mut f)?;

        // Decrypt the image data 解密图片数据
        let image_data = self.decrypt_image_data(&mut f)?;

        let file_name_prefix = Path::new(output_dir)
        .join(Path::new(file_path).file_stem().unwrap())
        .to_str()
        .unwrap()
        .to_string();

        // Write the image data to a file 将图片数据写入文件
        if target == DumpTarget::IMAGE || target == DumpTarget::ALL {
            let mut image_file = File::create(format!("{}.png", file_name_prefix))?;
            image_file.write_all(&image_data)?;
        }

        if target == DumpTarget::AUDIO || target == DumpTarget::ALL {
            // Decrypt the audio data and write to a file 解密音频数据并写入文件
            let audio_data = self.decrypt_file_data(&mut f, &key_box)?;
            let format = meta_data["format"].as_str().unwrap();
            let mut audio_file = File::create(format!("{}.{}", file_name_prefix, format))?;
            audio_file.write_all(&audio_data)?;
        }
        Ok(())
    }

    fn decrypt_key(&self, f: &mut File) -> io::Result<Vec<u8>> {
        // Move file pointer, skipping two bytes 移动文件指针，跳过两个字节
        f.seek(SeekFrom::Current(2))?;

        // Read key length using byteorder for little-endian u32 读取 key 长度，使用 byteorder 库来读取小端序的 u32
        let key_length = f.read_u32::<LittleEndian>()?;

        // Decrypt key data 调用 decrypt_key_data 方法
        let key_data = self.decrypt_key_data(f, key_length)?;

        // Generate key box 调用 generate_key_box 方法
        Ok(self.generate_key_box(&key_data))
    }

    fn decrypt_key_data(&self, f: &mut File, key_length: u32) -> io::Result<Vec<u8>> {
        // Read the key data 读取 key 数据
        let mut key_data = vec![0; key_length as usize];
        f.read_exact(&mut key_data)?;

        // XOR each byte with 0x64 对每个字节进行异或操作
        for byte in &mut key_data {
            *byte ^= 0x64;
        }

        // Create an AES decryptor instance 创建 AES 解密器实例
        let cipher = Ecb::<Aes128, Pkcs7>::new_from_slices(&self.core_key, Default::default())
            .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;

        // Decrypt the data 解密数据
        let decrypted_data = cipher
            .decrypt_vec(&key_data)
            .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;

        // Skip the first 17 bytes and return the rest 跳过前 17 个字节并返回剩余部分
        Ok(decrypted_data[17..].to_vec())
    }

    fn generate_key_box(&self, key_data: &[u8]) -> Vec<u8> {
        let key_length = key_data.len();
        let mut key_box: Vec<u8> = (0..=255).collect();
        let mut c: usize = 0;
        let mut last_byte: u8 = 0;
        let mut key_offset: usize = 0;

        for i in 0..256 {
            let swap = key_box[i];
            c = (swap as usize + last_byte as usize + key_data[key_offset] as usize) & 0xFF;
            key_offset += 1;
            if key_offset >= key_length {
                key_offset = 0;
            }
            key_box[i] = key_box[c];
            key_box[c] = swap;
            last_byte = c as u8;
        }

        key_box
    }

    fn decrypt_meta_data(&self, f: &mut File) -> io::Result<Value> {
        // Read the metadata length 读取元数据长度
        let meta_length = f.read_u32::<LittleEndian>()?;

        // Read the metadata 读取元数据
        let mut meta_data = vec![0; meta_length as usize];
        f.read_exact(&mut meta_data)?;

        // XOR operation for each byte 对每个字节进行异或操作
        for byte in &mut meta_data {
            *byte ^= 0x63;
        }

        // Base64 decode, skipping the first 22 bytes Base64 解码，跳过前 22 个字节
        meta_data =
            decode(&meta_data[22..]).map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;

        // Set up the AES decryptor 设置 AES 解密器
        let cipher = Ecb::<Aes128, Pkcs7>::new_from_slices(&self.meta_key, Default::default())
            .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;

        // Decrypt the data 解密数据
        let decrypted_data = cipher
            .decrypt_vec(&meta_data)
            .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;

        // Remove the first 6 bytes and convert to a UTF-8 string
        // 去除前 6 个字节并转换为 UTF-8 字符串
        let decrypted_str = String::from_utf8(decrypted_data[6..].to_vec())
            .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;

        // Parse JSON 解析 JSON
        serde_json::from_str(&decrypted_str).map_err(|e| io::Error::new(io::ErrorKind::Other, e))
    }

    fn decrypt_image_data(&self, f: &mut File) -> io::Result<Vec<u8>> {
        // Read the CRC32 value 读取 CRC32 值
        let crc32 = f.read_u32::<LittleEndian>()?;

        // Skip 5 bytes 跳过 5 个字节
        f.seek(SeekFrom::Current(5))?;

        // Read the image size 读取图片大小
        let image_size = f.read_u32::<LittleEndian>()?;

        // Read the image data 读取图片数据
        let mut image_data = vec![0; image_size as usize];
        f.read_exact(&mut image_data)?;

        Ok(image_data)
    }

    fn decrypt_file_data(&self, f: &mut File, key_box: &[u8]) -> io::Result<Vec<u8>> {
        let mut buffer = Vec::new();
        let mut chunk = [0u8; 0x8000];
        loop {
            let bytes_read = f.read(&mut chunk)?;
            if bytes_read == 0 {
                break;
            }
            // Decrypt the chunk of audio data 解密音频数据块
            for i in 0..bytes_read {
                let j = (i + 1) & 0xFF;
                chunk[i] ^= key_box[(key_box[j] as usize
                    + key_box[(key_box[j] as usize + j) & 0xFF] as usize)
                    & 0xFF];
            }
            buffer.extend_from_slice(&chunk[..bytes_read]);
        }
        Ok(buffer)
    }
}

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
    let ncm_dump = NcmDump::new();
    if let Err(e) = ncm_dump.dump(file_path, output_dir, target) {
        println!("Error: {}", e);
    }
}
