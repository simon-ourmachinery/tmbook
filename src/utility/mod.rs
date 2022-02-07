use flate2::read::GzDecoder;
use git2::Repository;
use reqwest::*;
use std::{
    fs::{self, ReadDir},
    io::{self, Cursor},
    path::{Path, PathBuf},
};
use tar::Archive;

pub const TM_BOOK_CODE_SNIPPETS: &str =
    "https://github.com/OurMachinery/themachinery-book-code-snippets";
pub const TM_BOOKS_REPO: &str = "https://github.com/OurMachinery/themachinery-books";
const TM_BOOK_BIN_DIR: &str = "./mdbook-bin";

pub fn get_mdbook_url() -> &'static str {
    if cfg!(windows) {
        "https://github.com/rust-lang/mdBook/releases/download/v0.4.15/mdbook-v0.4.15-x86_64-pc-windows-msvc.zip"
    } else {
        "https://github.com/rust-lang/mdBook/releases/download/v0.4.15/mdbook-v0.4.15-x86_64-unknown-linux-gnu.tar.gz"
    }
}
pub fn get_mdbook_toc_url() -> &'static str {
    if cfg!(windows) {
        "https://github.com/badboy/mdbook-toc/releases/download/0.8.0/mdbook-toc-0.8.0-x86_64-pc-windows-msvc.zip"
    } else {
        "https://github.com/badboy/mdbook-toc/releases/download/0.8.0/mdbook-toc-0.8.0-x86_64-unknown-linux-gnu.tar.gz"
    }
}
pub fn get_mdbook_linkcheck_url() -> &'static str {
    if cfg!(windows) {
        "https://github.com/Michael-F-Bryan/mdbook-linkcheck/releases/download/v0.7.6/mdbook-linkcheck.x86_64-pc-windows-msvc.zip"
    } else {
        "https://github.com/Michael-F-Bryan/mdbook-linkcheck/releases/download/v0.7.6/mdbook-linkcheck.x86_64-unknown-linux-gnu.zip"
    }
}

pub fn get_clang_format_url() -> &'static str {
    if cfg!(windows) {
        "https://ourmachinery.com/lib/clang-format-6.0.0-win64.zip"
    } else {
        "https://ourmachinery.com/lib/clang-format-6.0.0-linux.zip"
    }
}

pub fn get_mdbook() -> &'static str {
    if cfg!(windows) {
        "mdbook.exe"
    } else {
        "mdbook"
    }
}

pub fn get_clang_format(path: Option<&str>) -> PathBuf {
    if cfg!(windows) {
        get_bin_dir(path)
            .join("clang-format-6.0.0-win64")
            .join("clang-format-6.0.exe")
    } else {
        get_bin_dir(path)
            .join("clang-format-6.0.0-linux")
            .join("clang-format-6.0")
    }
}

pub fn check_and_set_or_download_book_code_snippets(code_snippets_path: &Path) {
    let path = std::env::var("TM_BOOK_CODE_SNIPPETS");
    if path.is_err() {
        println!(
            "Warning: Could not find: `TM_BOOK_CODE_SNIPPETS` trying to set the var automatically"
        );
        let path = code_snippets_path;
        if path.exists() {
            std::env::set_var(
                "TM_BOOK_CODE_SNIPPETS",
                std::fs::canonicalize(&path)
                    .unwrap()
                    .join("examples")
                    .as_os_str(),
            );
            println!(
                "TM_BOOK_CODE_SNIPPETS: {:?}",
                std::fs::canonicalize(&path)
                    .unwrap()
                    .join("examples")
                    .as_os_str()
            );
        } else {
            let url = TM_BOOK_CODE_SNIPPETS;
            match Repository::clone(url, code_snippets_path) {
                Ok(_) => {
                    std::env::set_var(
                        "TM_BOOK_CODE_SNIPPETS",
                        std::fs::canonicalize(&code_snippets_path)
                            .unwrap()
                            .join("examples")
                            .as_os_str(),
                    );
                    println!(
                        "TM_BOOK_CODE_SNIPPETS: {:?}",
                        std::fs::canonicalize(&code_snippets_path)
                            .unwrap()
                            .join("examples")
                            .as_os_str()
                    );
                }
                Err(_) => {
                    eprintln!("Cannot clone: {}", url);
                }
            };
        }
    }
}

pub fn find_bin_dir(current_dir: &PathBuf, search: &PathBuf) -> Option<PathBuf> {
    let paths = fs::read_dir(current_dir);
    if paths.is_err() {
        return None;
    }
    let paths = paths.unwrap();
    let process_path = |p: ReadDir| {
        for path in p {
            if path.is_ok() {
                let path = path.unwrap();
                if path.file_type().unwrap().is_dir() {
                    let file_name = path.file_name();
                    let current_dir = current_dir.join(&file_name);
                    let bin_dir = current_dir.join(search);
                    if !bin_dir.exists() {
                        let res = find_bin_dir(&current_dir, search);
                        if res.is_some() {
                            return res;
                        }
                    } else {
                        return Some(current_dir.join(search));
                    }
                }
            }
        }
        None
    };
    let res = process_path(paths);
    if res.is_none() {
        let parent = current_dir.parent().unwrap();
        let paths = fs::read_dir(parent).unwrap();
        let res = process_path(paths);
        res
    } else {
        res
    }
}

pub fn get_bin_dir(path: Option<&str>) -> PathBuf {
    let mut bin_dir = if path.is_none() {
        let env = std::env::var("TM_BOOK_BIN_DIR");
        match env {
            Err(_) => PathBuf::from(TM_BOOK_BIN_DIR),
            Ok(val) => PathBuf::from(&val),
        }
    } else {
        PathBuf::from(path.unwrap())
    };
    if !bin_dir.exists() {
        let cwd = std::env::current_dir().unwrap();
        let res = find_bin_dir(&cwd, &bin_dir);
        if res.is_some() {
            bin_dir = res.unwrap()
        } else {
            // search for a directory in a upper directory:
            std::fs::create_dir(bin_dir.as_path()).unwrap();
            eprintln!(
                "Could not find {:?}. Created folder for you.",
                bin_dir.as_path()
            );
        }
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

    if fname.extension().unwrap() == "zip" {
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
    } else {
        let tar = GzDecoder::new(file);
        let mut archive = Archive::new(tar);
        archive.unpack(bin_dir).unwrap();
    }
    Ok(())
}
