use {
    std::path::PathBuf,
    arrayref::{
        array_mut_ref,
        array_ref,
    },
    itermore::IterArrayChunks as _,
};

mod crc;

const DCMPSIZE: usize = 0x0400_0000;

struct Table {
    /// Start Virtual Address
    start_virt: u32,
    /// End Virtual Address
    end_virt: u32,
    /// Start Physical Address
    start_phys: u32,
    /// End Physical Address
    end_phys: u32,
}

fn find_table(in_rom: &[u8]) -> Result<usize, Error> {
    // Start at the end of the makerom (0x10600000)
    // Look for dma entry for the makeom
    // Should work for all Zelda64 titles
    for idx in (1048..).step_by(4) {
        if in_rom.len() < 4 * idx + 8 { break }
        if *array_ref![in_rom, 4 * idx, 4] == [0x00; 4] && *array_ref![in_rom, 4 * idx + 4, 4] == [0x00, 0x00, 0x10, 0x60] {
            return Ok(idx * 4)
        }
    }
    Err(Error::TableNotFound)
}

fn get_table_entry(in_rom: &[u8], tab_start: usize, idx: usize) -> Table {
    // First 32 bytes are VROM start address, next 32 are VROM end address
    // Next 32 bytes are Physical start address, last 32 are Physical end address
    Table {
        start_virt: u32::from_be_bytes(*array_ref![in_rom, tab_start + idx * 16, 4]),
        end_virt: u32::from_be_bytes(*array_ref![in_rom, tab_start + idx * 16 + 4, 4]),
        start_phys: u32::from_be_bytes(*array_ref![in_rom, tab_start + idx * 16 + 8, 4]),
        end_phys: u32::from_be_bytes(*array_ref![in_rom, tab_start + idx * 16 + 12, 4]),
    }
}

fn set_table_entry(out_rom: &mut [u8], tab_start: usize, idx: usize, table: Table) {
    *array_mut_ref![out_rom, tab_start + idx * 16, 4] = table.start_virt.to_be_bytes();
    *array_mut_ref![out_rom, tab_start + idx * 16 + 4, 4] = table.end_virt.to_be_bytes();
    *array_mut_ref![out_rom, tab_start + idx * 16 + 8, 4] = table.start_phys.to_be_bytes();
    *array_mut_ref![out_rom, tab_start + idx * 16 + 12, 4] = table.end_phys.to_be_bytes();
}

fn decompress_inner(source: &[u8], decomp: &mut [u8], decomp_size: u32) {
    let mut src_place = 0;
    let mut dst_place = 0;
    let mut dist: usize;
    let mut num_bytes: u32;
    let mut code_byte = 0u8; // dummy value because rustc doesn't see the initialization guarantee
    let mut byte1: u8;
    let mut byte2: u8;
    let mut bit_count = 0u8;

    let source = &source[0x10..];
    while (dst_place as u32) < decomp_size {
        // If there are no more bits to test, get a new byte
        if bit_count == 0 {
            code_byte = source[src_place];
            src_place += 1;
            bit_count = 8;
        }

        // If bit 7 is a 1, just copy 1 byte from source to destination
        // Else do some decoding
        if code_byte & 0x80 != 0 {
            decomp[dst_place] = source[src_place];
            dst_place += 1;
            src_place += 1;
        } else {
            // Get 2 bytes from source
            byte1 = source[src_place];
            src_place += 1;
            byte2 = source[src_place];
            src_place += 1;

            // Calculate distance to move in destination
            // And the number of bytes to copy
            dist = (usize::from(byte1 & 0xF) << 8) | usize::from(byte2);
            let mut copy_place = dst_place - (dist + 1);
            num_bytes = u32::from(byte1 >> 4);

            // Do more calculations on the number of bytes to copy
            if num_bytes == 0 {
                num_bytes = u32::from(source[src_place]) + 0x12;
                src_place += 1;
            } else {
                num_bytes += 2;
            }

            // Copy data from a previous point in destination
            // to current point in destination
            for _ in 0..num_bytes {
                decomp[dst_place] = decomp[copy_place];
                dst_place += 1;
                copy_place += 1;
            }
        }

        // Set up for the next read cycle */
        code_byte = code_byte << 1;
        bit_count -= 1;
    }
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)] TryFromInt(#[from] std::num::TryFromIntError),
    #[error(transparent)] Wheel(#[from] wheel::Error),
    #[error("{} is not the correct size", .0.display())]
    InputSize(PathBuf),
    #[error("couldn't find table")]
    TableNotFound,
}

pub fn decompress(in_rom: &mut [u8]) -> Result<Vec<u8>, Error> {
    // byte swap if needed
    if in_rom[0] == 0x37 {
        for [hi, lo] in in_rom.iter_mut().arrays() {
            (*hi, *lo) = (*lo, *hi);
        }
    }
    let mut out_rom = in_rom.to_owned();
    out_rom.resize(DCMPSIZE, 0);

    // Find table offsets
    let tab_start = find_table(&in_rom)?;
    let tab = get_table_entry(&in_rom, tab_start, 2);
    let tab_size = usize::try_from(tab.end_virt - tab.start_virt)?;
    let tab_count = tab_size / 16;

    // Set everything past the table in outROM to 0
    out_rom[tab.end_virt.try_into()?..].fill(0);

    for idx in 3..tab_count {
        let mut temp_tab = get_table_entry(&in_rom, tab_start, idx);
        let size = temp_tab.end_virt - temp_tab.start_virt;

        // dmaTable will have 0xFFFFFFFF if file doesn't exist
        if usize::try_from(temp_tab.start_phys)? >= DCMPSIZE || usize::try_from(temp_tab.end_phys)? > DCMPSIZE {
            continue
        }

        // Copy if uncompressed, uncompress otherwise
        if temp_tab.end_phys == 0x0000_0000 {
            out_rom.splice(usize::try_from(temp_tab.start_virt)?..usize::try_from(temp_tab.start_virt + size)?, in_rom[usize::try_from(temp_tab.start_phys)?..usize::try_from(temp_tab.start_phys + size)?].iter().copied());
        } else {
            decompress_inner(&in_rom[usize::try_from(temp_tab.start_phys)?..], &mut out_rom[usize::try_from(temp_tab.start_virt)?..], size);
        }

        // Clean up outROM's table
        temp_tab.start_phys = temp_tab.start_virt;
        temp_tab.end_phys = 0x0000_0000;
        set_table_entry(&mut out_rom, tab_start, idx, temp_tab);
    }

    // Fix the CRC before writing the ROM
    crc::fix_crc(&mut out_rom);

    Ok(out_rom)
}
