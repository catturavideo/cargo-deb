use std::fs;
use std::io::{self, Read, Write};
use std::os::unix::fs::OpenOptionsExt;
use tar::Header as TarHeader;
use tar::Builder as TarBuilder;
use tar::EntryType;

use config::Config;
use try::{failed, Try};
use archive::{copy_files, CHMOD_FILE, CHMOD_BIN_OR_DIR};

/// Generates the uncompressed control.tar archive
pub fn generate_archive(archive: &mut TarBuilder<Vec<u8>>, options: &Config, time: u64) {
    copy_files(archive, &options.assets, time);
    generate_copyright(archive, options, time);
}

/// Generates the copyright file from the license file and adds that to the tar archive.
fn generate_copyright(archive: &mut TarBuilder<Vec<u8>>, options: &Config, time: u64) {
    let mut copyright: Vec<u8> = Vec::new();
    write!(&mut copyright, "Upstream Name: {}\n", options.name).unwrap();
    write!(&mut copyright, "Source: {}\n", options.repository).unwrap();
    write!(&mut copyright, "Copyright: {}\n", options.copyright).unwrap();
    write!(&mut copyright, "License: {}\n", options.license).unwrap();
    options.license_file.get(0)
        // Fail if the path cannot be found and report that the license file argument is missing.
        .map_or_else(|| failed("cargo-deb: missing license file argument"), |path| {
            // Now we need to obtain the amount of lines to skip at the top of the file.
            let lines_to_skip = options.license_file.get(1)
                // If no argument is given, or if the argument is not a number, return 0.
                .map_or(0, |x| x.parse::<usize>().unwrap_or(0));
            // Now we need to attempt to open the file.
            let mut file = fs::File::open(path).try("cargo-deb: license file could not be opened");
            // The capacity of the file can be obtained from the metadata.
            let capacity = file.metadata().ok().map_or(0, |x| x.len());
            // We are going to store the contents of the license file in a single string with the size of file.
            let mut license_string = String::with_capacity(capacity as usize);
            // Attempt to read the contents of the license file into the license string.
            file.read_to_string(&mut license_string).try("cargo-deb: error reading license file");
            // Skip the first `A` number of lines and then iterate each line after that.
            for line in license_string.lines().skip(lines_to_skip) {
                // If the line is empty, write a dot, else write the line.
                if line.is_empty() {
                    copyright.write(b".\n").unwrap();
                } else {
                    copyright.write(line.trim().as_bytes()).unwrap();
                    copyright.write(b"\n").unwrap();
                }
            }
        });

    // Write a copy to the disk for the sake of obtaining a md5sum for the control archive.
    let mut file = fs::OpenOptions::new().create(true).write(true).truncate(true).mode(CHMOD_FILE)
        .open("target/debian/copyright").unwrap_or_else(|err| {
            failed(format!("cargo-deb: unable to open copyright file for writing: {}", err.to_string()));
        });
    file.write_all(copyright.as_slice()).try("cargo-deb: unable to write copyright file to disk");
    let target = String::from("./usr/share/doc/") + &options.name + "/";

    for dir in &[".", "./usr/", "./usr/share/", "./usr/share/doc/", target.as_str()] {
        let mut header = TarHeader::new_gnu();
        header.set_mtime(time);
        header.set_size(0);
        header.set_mode(CHMOD_BIN_OR_DIR);
        header.set_path(&dir).unwrap();
        header.set_entry_type(EntryType::Directory);
        header.set_cksum();
        archive.append(&header, &mut io::empty()).unwrap();
    }

    // Now add a copy to the archive
    let mut header = TarHeader::new_gnu();
    header.set_mtime(time);
    header.set_path(&(target + "copyright")).unwrap();
    header.set_size(copyright.len() as u64);
    header.set_mode(CHMOD_FILE);
    header.set_cksum();
    archive.append(&header, copyright.as_slice()).try("cargo-deb: unable to append copyright");
}
