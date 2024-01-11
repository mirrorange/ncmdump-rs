use aes::Aes128;
use base64::prelude::*;
use block_modes::block_padding::Pkcs7;
use block_modes::{BlockMode, Ecb};
use byteorder::{LittleEndian, ReadBytesExt};
use hex::FromHex;
use serde_json::Value;
use std::fs::File;
use std::io::{self, prelude::*, Seek, SeekFrom};
use std::path::Path;
use std::ffi::CStr;
use std::os::raw::c_char;

const CORE_KEY: &str = "687A4852416D736F356B496E62617857";
const META_KEY: &str = "2331346C6A6B5F215C5D2630553C2728";

#[derive(PartialEq, Eq)]
pub enum DumpTarget {
    ALL,
    AUDIO,
    IMAGE,
}


#[no_mangle]
pub extern "C" fn dump_wrapper(file_path: *const c_char, output_dir: *const c_char, target: i32) -> i32 {
    unsafe {
        assert!(!file_path.is_null());
        assert!(!output_dir.is_null());

        let file_path_cstr = CStr::from_ptr(file_path);
        let output_dir_cstr = CStr::from_ptr(output_dir);

        let file_path = match file_path_cstr.to_str() {
            Ok(s) => s,
            Err(_) => return 1,
        };

        let output_dir = match output_dir_cstr.to_str() {
            Ok(s) => s,
            Err(_) => return 1,
        };

        let target = match target {
            0 => DumpTarget::ALL,
            1 => DumpTarget::AUDIO,
            2 => DumpTarget::IMAGE,
            _ => return 1,
        };

        match dump(file_path, output_dir, target) {
            Ok(_) => 0,
            Err(_) => 1,
        }
    }
}

#[no_mangle]
pub fn dump(file_path: &str, output_dir: &str, target: DumpTarget) -> io::Result<()> {
    // Open the .ncm file 打开 .ncm 文件
    let mut f = open_ncm_file(file_path)?;

    // Decrypt the key 解密 key
    let key_box = decrypt_key(&mut f)?;

    // Decrypt the metadata 解密元数据
    let meta_data = decrypt_meta_data(&mut f)?;

    // Decrypt the image data 解密图片数据
    let image_data = decrypt_image_data(&mut f)?;

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
        let audio_data = decrypt_file_data(&mut f, &key_box)?;
        let format = meta_data["format"].as_str().unwrap();
        let mut audio_file = File::create(format!("{}.{}", file_name_prefix, format))?;
        audio_file.write_all(&audio_data)?;
    }
    Ok(())
}

fn open_ncm_file(file_path: &str) -> io::Result<File> {
    let mut f = File::open(file_path)?;

    // Check the file header 检查文件头
    let mut header = [0; 8];
    f.read_exact(&mut header)?;
    if &header != b"CTENFDAM" {
        return Err(io::Error::new(io::ErrorKind::Other, "Invalid file header"));
    }

    Ok(f)
}

fn decrypt_key(f: &mut File) -> io::Result<Vec<u8>> {
    // Move file pointer, skipping two bytes 移动文件指针，跳过两个字节
    f.seek(SeekFrom::Current(2))?;

    // Read key length using byteorder for little-endian u32 读取 key 长度，使用 byteorder 库来读取小端序的 u32
    let key_length = f.read_u32::<LittleEndian>()?;

    // Decrypt key data 调用 decrypt_key_data 方法
    let key_data = decrypt_key_data(f, key_length)?;

    // Generate key box 调用 generate_key_box 方法
    Ok(generate_key_box(&key_data))
}

fn decrypt_key_data(f: &mut File, key_length: u32) -> io::Result<Vec<u8>> {
    // Read the key data 读取 key 数据
    let mut key_data = vec![0; key_length as usize];
    f.read_exact(&mut key_data)?;

    // XOR each byte with 0x64 对每个字节进行异或操作
    for byte in &mut key_data {
        *byte ^= 0x64;
    }

    // Create an AES decryptor instance 创建 AES 解密器实例
    let core_key = Vec::from_hex(CORE_KEY).unwrap();
    let cipher = Ecb::<Aes128, Pkcs7>::new_from_slices(&core_key, Default::default())
        .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;

    // Decrypt the data 解密数据
    let decrypted_data = cipher
        .decrypt_vec(&key_data)
        .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;

    // Skip the first 17 bytes and return the rest 跳过前 17 个字节并返回剩余部分
    Ok(decrypted_data[17..].to_vec())
}

fn generate_key_box(key_data: &[u8]) -> Vec<u8> {
    let key_length = key_data.len();
    let mut key_box: Vec<u8> = (0..=255).collect();
    let mut last_byte: u8 = 0;
    let mut key_offset: usize = 0;

    for i in 0..256 {
        let swap = key_box[i];
        let c = (swap as usize + last_byte as usize + key_data[key_offset] as usize) & 0xFF;
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

fn decrypt_meta_data(f: &mut File) -> io::Result<Value> {
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
    meta_data = BASE64_STANDARD.decode(&meta_data[22..]).map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;

    // Set up the AES decryptor 设置 AES 解密器
    let meta_key = Vec::from_hex(META_KEY).unwrap();
    let cipher = Ecb::<Aes128, Pkcs7>::new_from_slices(&meta_key, Default::default())
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

fn decrypt_image_data(f: &mut File) -> io::Result<Vec<u8>> {
    // Read the CRC32 value 读取 CRC32 值
    let _crc32 = f.read_u32::<LittleEndian>()?;

    // Skip 5 bytes 跳过 5 个字节
    f.seek(SeekFrom::Current(5))?;

    // Read the image size 读取图片大小
    let image_size = f.read_u32::<LittleEndian>()?;

    // Read the image data 读取图片数据
    let mut image_data = vec![0; image_size as usize];
    f.read_exact(&mut image_data)?;

    Ok(image_data)
}

fn decrypt_file_data(f: &mut File, key_box: &[u8]) -> io::Result<Vec<u8>> {
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