use reqwest::*;
use std::{
    fs,
    io::{self, Cursor},
    path::{Path, PathBuf},
};

pub const MDBOOK_URL : &str = "https://github.com/rust-lang/mdBook/releases/download/v0.4.15/mdbook-v0.4.15-x86_64-pc-windows-msvc.zip";
pub const MDBOOK_TOC_URL : &str = "https://github.com/badboy/mdbook-toc/releases/download/0.8.0/mdbook-toc-0.8.0-x86_64-pc-windows-msvc.zip";
pub const MDBOOK_LINKCHECK_URL : &str = "https://github.com/Michael-F-Bryan/mdbook-linkcheck/releases/download/v0.7.6/mdbook-linkcheck.x86_64-pc-windows-msvc.zip";
pub const CLANGFORMAT_URL: &str = "https://ourmachinery.com/lib/clang-format-6.0.0-win64.zip";
pub const TM_BOOK_CODE_SNIPPETS: &str =
    "https://github.com/OurMachinery/themachinery-book-code-snippets";
pub const TM_BOOKS_REPO: &str = "https://github.com/OurMachinery/themachinery-books";
const TM_BOOK_BIN_DIR: &str = "./mdbook-bin";

pub fn get_bin_dir(path: Option<&str>) -> PathBuf {
    let bin_dir = if path.is_none() {
        let env = std::env::var("TM_BOOK_BIN_DIR");
        match env {
            Err(_) => PathBuf::from(TM_BOOK_BIN_DIR),
            Ok(val) => PathBuf::from(&val),
        }
    } else {
        PathBuf::from(path.unwrap())
    };

    if !bin_dir.exists() {
        std::fs::create_dir(bin_dir.as_path()).unwrap();
        eprintln!(
            "Could not find {:?}. Created folder for you.",
            bin_dir.as_path()
        );
    }

    bin_dir
}

// see https://georgik.rocks/how-to-download-binary-file-in-rust-by-reqwest/
pub async fn fetch_url(url: String, file_name: &std::path::PathBuf) -> Result<()> {
    let response = reqwest::get(url).await?;
    let mut file = std::fs::File::create(file_name).unwrap();
    let mut content = Cursor::new(response.bytes().await?);
    std::io::copy(&mut content, &mut file).unwrap();
    Ok(())
}

pub fn unzip(fname: &Path, bin_dir: &Path) -> Result<()> {
    let file = fs::File::open(fname).unwrap();
    let mut archive = zip::ZipArchive::new(file).unwrap();
    for i in 0..archive.len() {
        let mut file = archive.by_index(i).unwrap();
        let mut outpath = match file.enclosed_name() {
            Some(path) => path.to_owned(),
            None => continue,
        };
        outpath = bin_dir.join(outpath);

        if (*file.name()).ends_with('/') {
            println!("File {} extracted to \"{}\"", i, outpath.display());
            fs::create_dir_all(&outpath).unwrap();
        } else {
            println!(
                "File {} extracted to \"{}\" ({} bytes)",
                i,
                outpath.display(),
                file.size()
            );
            if let Some(p) = outpath.parent() {
                if !p.exists() {
                    fs::create_dir_all(&p).unwrap();
                }
            }
            let mut outfile = fs::File::create(&outpath).unwrap();
            io::copy(&mut file, &mut outfile).unwrap();
        }
    }
    Ok(())
}
