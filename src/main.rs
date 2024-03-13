
use goblin::elf::{header::*, section_header::*, Elf};
use memmap2::Mmap;
use std::env;
use std::fs::File;
use std::io::{self, Write};
use std::process;
use procinfo::pid::{statm_self, Statm};

fn memuse() {
    match statm_self() {
        Ok(statm) => {
            println!("Memory used by the process: {} KB", statm.size);
        }
        Err(e) => {
            eprintln!("Error: {}", e);
        }
    }
}

fn replace_bytes(data: &mut [u8], src: &[u8], dest: &[u8]) {
    let src_len = src.len();
    let dest_len = dest.len();

    for i in 0..data.len() - src_len + 1 {
        if &data[i..i + src_len] == src {
            data[i..i + dest_len].copy_from_slice(dest);
        }
    }
}

/*
fn replace_bytes(data: &mut [u8], src: &[u8], dest: &[u8]) {
    for chunk in data.windows(src.len()) {
        if chunk == src {
            chunk.copy_from_slice(dest);
        }
    }
}
*/

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() != 4 {
        writeln!(io::stderr(), "Usage: {} <elf_file> <src> <dst>", args[0]).unwrap();
        process::exit(1);
    }

    let elf_file_path = &args[1];
    let src_bytes = args[2].as_bytes();
    let dest_bytes = args[3].as_bytes();

    let elf_file = match File::options().read(true).write(true).open(elf_file_path) {
        Ok(f) => f,
        Err(e) => {
            writeln!(io::stderr(), "Error opening file: {}", e).unwrap();
            process::exit(1);
        }
    };

    memuse();
    let mmap = unsafe { Mmap::map(&elf_file).unwrap() };
    memuse();

    match Elf::parse(&mmap) {
        Ok(elf) => {
            memuse();
            println!("Section headers:");
            for header in elf.section_headers.iter() {
                let name = elf.shdr_strtab.get_at(header.sh_name).unwrap_or("<invalid utf-8>");
                println!(
                    "Name: {:?}, Address: {:#x}, Size: {:#x}",
                    name,
                    header.sh_addr,
                    header.sh_size
                );
            }
        }
        Err(e) => {
            writeln!(io::stderr(), "Error parsing ELF file: {}", e).unwrap();
            process::exit(1);
        }
    }

    replace_bytes(&mut mmap.make_mut().unwrap(), src_bytes, dest_bytes);
    //mmap.make_mut().unwrap().flush_async();
}

