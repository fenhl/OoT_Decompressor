use {
    std::path::PathBuf,
    wheel::traits::IoResultExt as _,
    decompress::{
        Error,
        decompress,
    },
};

const COMPSIZE: usize = 0x0200_0000;

#[derive(clap::Parser)]
struct Args {
    input_rom: PathBuf,
    output_rom: Option<PathBuf>,
}

#[wheel::main]
fn main(Args { input_rom, output_rom }: Args) -> Result<(), Error> {
    // If no output file was specified, make one
    // Add "-decomp.z64" to the end of the input file
    let output_rom = output_rom.unwrap_or_else(|| {
        let stem = input_rom.file_stem().unwrap_or_default().to_string_lossy();
        input_rom.with_file_name(format!("{stem}-decomp.z64"))
    });

    let mut in_rom = std::fs::read(&input_rom).at(&input_rom)?;
    if in_rom.len() != COMPSIZE {
        return Err(Error::InputSize(input_rom))
    }
    let out_rom = decompress(&mut in_rom)?;
    std::fs::write(&output_rom, out_rom).at(output_rom)?;

    Ok(())
}
