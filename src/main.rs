
use goblin::elf::Elf;
use memmap2::Mmap;
use std::env;
use std::fs::File;
use std::io::{self, Write};
use std::process;

fn replace_bytes(data: &mut [u8], src: &[u8], dst: &[u8]) {
    let src_len = src.len();
    let dst_len = dst.len();

    for i in 0..data.len() - src_len + 1 {
        if &data[i..i + src_len] == src {
            data[i..i + dst_len].copy_from_slice(dst);
        }
    }
}

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() != 4 {
        writeln!(io::stdout(), "Usage: {} <elf_file> <src> <dst>", args[0]).unwrap();
        process::exit(1);
    }

    let elf_file_path = &args[1];
    let src_bytes = args[2].as_bytes();
    let dst_bytes = args[3].as_bytes();

    if src_bytes.len() != dst_bytes.len() {
        writeln!(io::stdout(), "source and destination strings should have the same length in bytes, instead `{}' has {} bytes but `{}' has {} bytes",
                 args[2], src_bytes.len(), args[3], dst_bytes.len()).unwrap();
	process::exit(1);
    }

    let elf_file = match File::options().read(true).write(true).open(elf_file_path) {
        Ok(f) => f,
        Err(e) => {
            writeln!(io::stdout(), "Error opening file `{elf_file_path}': {e}").unwrap();
            process::exit(1);
        }
    };

    let mmap = unsafe { Mmap::map(&elf_file).unwrap() };

    let prefix_list = vec![
        ".rodata",
        ".debug_line",
        ".debug_str",
    ];
    
    let mut sections = Vec::new(); 
    match Elf::parse(&mmap) {
        Ok(elf) => {
    	    for header in elf.section_headers.iter() {
                let name = elf.shdr_strtab.get_at(header.sh_name).unwrap_or("<invalid utf-8>");
                let name_matches = prefix_list.iter().any(|prefix| name.starts_with(prefix));
                if name_matches {
	            sections.push(header.clone());
                }
	    }
	},
        Err(e) => {
            writeln!(io::stdout(), "Error parsing ELF file `{elf_file_path}': {e}").unwrap();
            process::exit(1);
        }
    };

    let mut mmap_mut = mmap.make_mut().unwrap();
    for header in sections {
        let offset = header.sh_offset as usize;
        let size = header.sh_size as usize;
        replace_bytes(&mut mmap_mut[offset .. offset+size], src_bytes, dst_bytes);
    }
}

