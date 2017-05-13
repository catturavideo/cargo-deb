use std::fs;
use std::io::{self, Read};
use std::path::Path;
use itertools::Itertools;
use tar::Header as TarHeader;
use tar::Builder as TarBuilder;
use tar::EntryType;

use try::Try;

pub const CHMOD_FILE:       u32 = 420;
pub const CHMOD_BIN_OR_DIR: u32 = 493;

/// Copies all the files to be packaged into the tar archive.
pub fn copy_files(archive: &mut TarBuilder<Vec<u8>>, assets: &Vec<Vec<String>>, time: u64) {
    let mut added_directories: Vec<String> = Vec::new();
    for asset in assets {
        // Collect the source and target paths
        let origin = asset.get(0).try("cargo-deb: unable to get asset's path");
        let mut target = String::from("./") + asset.get(1).try("cargo-deb: unable to get asset's target");
        let chmod = asset.get(2).map(|x| u32::from_str_radix(x, 8).unwrap())
            .try("cargo-deb: unable to get chmod argument");
        if target.chars().next().unwrap() == '/' { target.remove(0); }
        if target.chars().last().unwrap() == '/' {
            target.push_str(Path::new(origin).file_name().unwrap().to_str().unwrap());
        }

        // Append each of the directories found in the file's pathname to the archive before adding the file
        target.char_indices()
            // Exclusively search for `/` characters only
            .filter(|&(_, character)| character == '/')
            // Use the indexes of the `/` characters to collect a list of directory pathnames
            .map(|(id, _)| &target[0..id+1])
            // For each directory pathname found, attempt to add it to the list of directories
            .foreach(|directory| {
                if !added_directories.iter().any(|x| x.as_str() == directory) {
                    added_directories.push(directory.to_owned());
                    let mut header = TarHeader::new_gnu();
                    header.set_mtime(time);
                    header.set_size(0);
                    header.set_mode(CHMOD_BIN_OR_DIR);
                    header.set_path(&directory).unwrap();
                    header.set_entry_type(EntryType::Directory);
                    header.set_cksum();
                    archive.append(&header, &mut io::empty()).unwrap();
                }
            });

        // Add the file to the archive
        let mut file = fs::File::open(&origin).try("cargo-deb: unable to open file");
        let capacity = file.metadata().ok().map_or(0, |x| x.len()) as usize;
        let mut out_data: Vec<u8> = Vec::with_capacity(capacity);
        file.read_to_end(&mut out_data).try("cargo-deb: unable to read asset's data");
        let mut header = TarHeader::new_gnu();
        header.set_mtime(time);
        header.set_path(&target).unwrap();
        header.set_mode(chmod);
        header.set_size(capacity as u64);
        header.set_cksum();
        archive.append(&header, out_data.as_slice()).try("cargo-deb: unable to write data to archive.");
    }
}
