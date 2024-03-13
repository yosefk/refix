
use goblin::elf::{header::*, section_header::*, Elf};
use memmap::Mmap;
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

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() != 2 {
        writeln!(io::stderr(), "Usage: {} <elf_file>", args[0]).unwrap();
        process::exit(1);
    }

    let elf_file_path = &args[1];
    let elf_file = match File::open(elf_file_path) {
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
                let name = elf.shdr_strtab.get(header.sh_name).unwrap_or(Ok("<invalid utf-8>"));
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
}

