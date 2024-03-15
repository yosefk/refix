
use goblin::elf::Elf;
use memmap2::Mmap;
use std::env;
use std::fs::File;
use memchr::memmem;

macro_rules! fail {
    ($($arg:tt)*) => {{
        println!($($arg)*);
        std::process::exit(1);
    }};
}

enum FileType {
    ELF,
    AR,
    UNKNOWN,
}

//from /usr/include/{elf,ar}.h
const ELFMAG: &[u8; 4] = b"\x7fELF";
const ARMAG: &[u8; 8] = b"!<arch>\n";
const ARFMAG: &[u8; 2] = b"`\n";

fn detect_mapped_file_type(data: &[u8]) -> FileType {
    if data.starts_with(ELFMAG) {
        FileType::ELF
    } else if data.starts_with(ARMAG) {
        FileType::AR
    } else {
        FileType::UNKNOWN
    }
}

//in ELF files, we look for sections with string constants (.rodata or .rodata.*)
//where __FILE__ goes, or DWARF sections where file paths go (.debug_line and .debug_str),
//and copy their headers (to use the offset and size later).
//
//goblin gives us the ELF section headers which have file offsets [byte offsets
//for mmaped files] in the sh_offset field, so we needn't copy data but can work in-place easily
fn get_elf_sections_to_replace_in(
    data: &[u8],
    sections: &mut Vec<goblin::elf::SectionHeader>,
    offset: u64,
) {
    let prefix_list = vec![".rodata", ".debug_line", ".debug_str"];

    match Elf::parse(data) {
        Ok(elf) => {
            for header in elf.section_headers.iter() {
                let name = elf.shdr_strtab.get_at(header.sh_name).unwrap_or(
                    "<invalid utf-8>",
                );
                let name_matches = prefix_list.iter().any(|prefix| name.starts_with(prefix));
                if name_matches {
                    let mut h = header.clone();
                    h.sh_offset += offset;
                    sections.push(h);
                }
            }
        }
        Err(e) => fail!("Error parsing ELF file: {e}"),
    };
}

//unfortunately the ar crate doesn't seem to provide offsets within the archive so there's no
//way to work in-place on the mmaped byte array thru the ar crate. instead, we parse the archive
//directly, based on the layout as documented in /usr/include/ar.h:
//
//struct ar_hdr
//  {
//    char ar_name[16];		/* Member file name, sometimes / terminated. */
//    char ar_date[12];		/* File date, decimal seconds since Epoch.  */
//    char ar_uid[6], ar_gid[6];	/* User and group IDs, in ASCII decimal.  */
//    char ar_mode[8];		/* File mode, in ASCII octal.  */
//    char ar_size[10];		/* File size, in ASCII decimal.  */
//    char ar_fmag[2];		/* Always contains ARFMAG.  */
//  };
//
fn get_elf_sections_to_replace_in_from_ar(
    data: &[u8],
    sections: &mut Vec<goblin::elf::SectionHeader>,
) {
    let ar_hdr_size = 60;
    let ar_size_offset = 48;
    let ar_size_len = 10;

    let mut pos = ARMAG.len();
    while pos < data.len() {
        if pos + ar_hdr_size > data.len() {
            fail!("archive truncated within an ar_hdr struct");
        }
        let hdr = &data[pos..pos + ar_hdr_size];
        if !hdr.ends_with(ARFMAG) {
            fail!("archive has a corrupted ar_hdr - ARFMAG not found");
        }
        let size = &hdr[ar_size_offset..ar_size_offset + ar_size_len];
        let str_size = std::str::from_utf8(size)
            .expect("Invalid UTF-8 in ar_hdr ar_size field")
            .trim();
        let int_size: usize = str_size.parse().expect(
            "ar_hdr ar_size field is not a decimal integer",
        );

        //let name = std::str::from_utf8(&hdr[0..16])
        //.expect("Invalid UTF-8 in ar_hdr ar_name field").trim();
        //println!("{name} {str_size}");

        pos += ar_hdr_size;

        if pos + int_size > data.len() {
            fail!("archive has a file with an end offset past the archive size");
        }

        let file_data = &data[pos..pos + int_size];
        match detect_mapped_file_type(file_data) {
            FileType::ELF => get_elf_sections_to_replace_in(&file_data, sections, pos as u64),
            _ => (),
        }

        pos += int_size;
    }
}

fn replace_bytes(data: &mut [u8], finder: &memmem::Finder, dst: &[u8]) -> bool {
    let len = dst.len();

    let mut replaced = false;
    let mut i: usize = 0;
    let end = data.len() - len;

    while i <= end {
        match finder.find(&data[i..]) {
            Some(pos) => {
                i += pos;
                data[i..i + len].copy_from_slice(dst);
                replaced = true;
                i += len;
            }
            None => {
                return replaced;
            }
        }
    }
    replaced
}

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() != 4 {
        fail!("Usage: {} <elf_file> <src> <dst>", args[0]);
    }

    let elf_file_path = &args[1];
    let src = &args[2];
    let dst = &args[3];
    let src_bytes = src.as_bytes();
    let dst_bytes = dst.as_bytes();

    if src_bytes.len() != dst_bytes.len() {
        fail!(
            "source and destination strings should have the same length in bytes, \
instead `{}' has {} bytes but `{}' has {} bytes",
            src,
            src_bytes.len(),
            dst,
            dst_bytes.len()
        );
    }

    let elf_file = match File::options().read(true).write(true).open(elf_file_path) {
        Ok(f) => f,
        Err(e) => fail!("Error opening file `{elf_file_path}': {e}"),
    };

    let mmap = unsafe { Mmap::map(&elf_file).expect("error mmaping the file") };
    let file_type = detect_mapped_file_type(&mmap);
    let mut sections = Vec::<goblin::elf::SectionHeader>::new();

    match file_type {
        FileType::ELF => get_elf_sections_to_replace_in(&mmap, &mut sections, 0),
        FileType::AR => get_elf_sections_to_replace_in_from_ar(&mmap, &mut sections),
        FileType::UNKNOWN => fail!("unknown file type (neither ELF nor ar archive)"),
    };

    let mut mmap_mut = mmap.make_mut().expect("error getting a mutable mmap");
    let finder = memmem::Finder::new(src_bytes);
    for header in sections {
        let offset = header.sh_offset as usize;
        let size = header.sh_size as usize;
        let replaced = replace_bytes(&mut mmap_mut[offset..offset + size], &finder, &dst_bytes);
        if replaced {
            mmap_mut.flush_async_range(offset, size).expect(
                "error flushing file changes",
            );
        }
    }
}
