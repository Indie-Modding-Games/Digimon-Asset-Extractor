//! The command that writes the PNG of the TIM files, from the end to end.

use dw_extract::cli::{run, Cli, Commands};
use std::path::{Path, PathBuf};

/// A TIM file with a CLUT of one palette and a 4-bit image of 16x16 pixels.
fn tim_4bpp() -> Vec<u8> {
    let mut data = Vec::new();

    // The header: the magic word and the flags (4-bit with a CLUT).
    data.extend_from_slice(&0x10u32.to_le_bytes());
    data.extend_from_slice(&0x08u32.to_le_bytes());

    // The CLUT: the length, the position, the size and the colours.
    let clut_data_size = 16 * 2;
    data.extend_from_slice(&((12 + clut_data_size) as u32).to_le_bytes());
    data.extend_from_slice(&256u16.to_le_bytes()); // x
    data.extend_from_slice(&480u16.to_le_bytes()); // y
    data.extend_from_slice(&16u16.to_le_bytes()); // width
    data.extend_from_slice(&1u16.to_le_bytes()); // height
    for color in 0..16u16 {
        data.extend_from_slice(&color.to_le_bytes());
    }

    // The image: 4 units of VRAM wide and 16 high, which is 16x16 pixels.
    let image_data_size = 4 * 2 * 16;
    data.extend_from_slice(&((12 + image_data_size) as u32).to_le_bytes());
    data.extend_from_slice(&0u16.to_le_bytes()); // x
    data.extend_from_slice(&0u16.to_le_bytes()); // y
    data.extend_from_slice(&4u16.to_le_bytes()); // width
    data.extend_from_slice(&16u16.to_le_bytes()); // height
    for index in 0..image_data_size {
        data.push((index % 16) as u8);
    }

    data
}

/// A TIM file with a CLUT of two palettes.
fn tim_two_palettes() -> Vec<u8> {
    let mut data = Vec::new();

    data.extend_from_slice(&0x10u32.to_le_bytes());
    data.extend_from_slice(&0x08u32.to_le_bytes());

    let clut_data_size = 32 * 2;
    data.extend_from_slice(&((12 + clut_data_size) as u32).to_le_bytes());
    data.extend_from_slice(&256u16.to_le_bytes());
    data.extend_from_slice(&480u16.to_le_bytes());
    data.extend_from_slice(&32u16.to_le_bytes());
    data.extend_from_slice(&1u16.to_le_bytes());
    for color in 0..32u16 {
        data.extend_from_slice(&color.to_le_bytes());
    }

    let image_data_size = 4 * 2 * 16;
    data.extend_from_slice(&((12 + image_data_size) as u32).to_le_bytes());
    data.extend_from_slice(&0u16.to_le_bytes());
    data.extend_from_slice(&0u16.to_le_bytes());
    data.extend_from_slice(&4u16.to_le_bytes());
    data.extend_from_slice(&16u16.to_le_bytes());
    for index in 0..image_data_size {
        data.push((index % 16) as u8);
    }

    data
}

/// A directory of its own for the test, under the temporary directory of the system.
fn temp_dir(name: &str) -> PathBuf {
    let mut dir = std::env::temp_dir();
    dir.push(format!("dw-extract-test-{}", name));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn convert(input: &Path, output: &Path, recursive: bool) -> Result<(), dw_extract::error::DwExtractError> {
    let cli = Cli {
        command: Commands::ConvertTim {
            input: input.to_path_buf(),
            output: output.to_path_buf(),
            recursive,
        },
    };
    run(cli)
}

/// The PNG files of a directory, sorted by name.
fn pngs(dir: &Path) -> Vec<String> {
    let mut files: Vec<String> = std::fs::read_dir(dir)
        .unwrap()
        .map(|entry| entry.unwrap().file_name().to_string_lossy().to_string())
        .filter(|name| name.ends_with(".png"))
        .collect();
    files.sort();
    files
}

#[test]
fn a_tim_gives_a_png() {
    let dir = temp_dir("tim-single");
    let input = dir.join("image.tim");
    std::fs::write(&input, tim_4bpp()).unwrap();

    let output = dir.join("out");
    convert(&input, &output, false).unwrap();

    assert_eq!(pngs(&output), vec!["image.png".to_string()]);

    // The PNG has the size of the image in pixels.
    let png = std::fs::read(output.join("image.png")).unwrap();
    let width = u32::from_be_bytes([png[16], png[17], png[18], png[19]]);
    let height = u32::from_be_bytes([png[20], png[21], png[22], png[23]]);
    assert_eq!(width, 16);
    assert_eq!(height, 16);
}

#[test]
fn a_tim_with_two_palettes_gives_a_png_each() {
    let dir = temp_dir("tim-palettes");
    let input = dir.join("image.tim");
    std::fs::write(&input, tim_two_palettes()).unwrap();

    let output = dir.join("out");
    convert(&input, &output, false).unwrap();

    assert_eq!(
        pngs(&output),
        vec!["image_p0.png".to_string(), "image_p1.png".to_string()]
    );
}

#[test]
fn a_directory_gives_a_png_of_each_file() {
    let dir = temp_dir("tim-directory");
    let input = dir.join("in");
    std::fs::create_dir_all(&input).unwrap();
    std::fs::write(input.join("one.tim"), tim_4bpp()).unwrap();
    std::fs::write(input.join("two.tim"), tim_4bpp()).unwrap();

    let output = dir.join("out");
    convert(&input, &output, false).unwrap();

    assert_eq!(
        pngs(&output),
        vec!["one.png".to_string(), "two.png".to_string()]
    );
}

#[test]
fn the_recursive_option_reads_the_subdirectories() {
    let dir = temp_dir("tim-recursive");
    let input = dir.join("in");
    std::fs::create_dir_all(input.join("sub")).unwrap();
    std::fs::write(input.join("one.tim"), tim_4bpp()).unwrap();
    std::fs::write(input.join("sub/two.tim"), tim_4bpp()).unwrap();

    let output = dir.join("out");
    convert(&input, &output, true).unwrap();

    assert_eq!(
        pngs(&output),
        vec!["one.png".to_string(), "two.png".to_string()]
    );
}

#[test]
fn without_the_recursive_option_the_subdirectories_are_left_alone() {
    let dir = temp_dir("tim-flat");
    let input = dir.join("in");
    std::fs::create_dir_all(input.join("sub")).unwrap();
    std::fs::write(input.join("one.tim"), tim_4bpp()).unwrap();
    std::fs::write(input.join("sub/two.tim"), tim_4bpp()).unwrap();

    let output = dir.join("out");
    convert(&input, &output, false).unwrap();

    assert_eq!(pngs(&output), vec!["one.png".to_string()]);
}

#[test]
fn a_broken_tim_fails_the_command() {
    let dir = temp_dir("tim-broken");
    let input = dir.join("in");
    std::fs::create_dir_all(&input).unwrap();
    std::fs::write(input.join("one.tim"), tim_4bpp()).unwrap();
    // A file with the magic word of a TIM, but with a block that does not fit.
    let mut broken = tim_4bpp();
    broken.truncate(20);
    std::fs::write(input.join("broken.tim"), broken).unwrap();

    let output = dir.join("out");
    assert!(convert(&input, &output, false).is_err());

    // The good file is still written.
    assert_eq!(pngs(&output), vec!["one.png".to_string()]);
}
