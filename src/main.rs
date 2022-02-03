use authors::Authors;
use auto_doc::AutoDoc;
use auto_include::AutoInclude;
use clap::{App, Arg, ArgMatches};
use git2::Repository;
use mdbook::errors::Error;
use mdbook::preprocess::{CmdPreprocessor, Preprocessor};
use std::path::{Path, PathBuf};
use std::process::{self, Stdio};
use std::{fs, io};
use utility::{get_bin_dir, CLANGFORMAT_URL, MDBOOK_LINKCHECK_URL, MDBOOK_TOC_URL, MDBOOK_URL};

use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;

mod authors;
mod auto_doc;
mod auto_include;
mod replace_path;
mod utility;
use replace_path::ReplacePaths;

use crate::utility::{fetch_url, unzip, TM_BOOKS_REPO, TM_BOOK_CODE_SNIPPETS};

#[derive(PartialEq)]
enum PreType {
    AutoDoc,
    ReplacePaths,
    AutoInclude,
    Authors,
}

pub fn make_app() -> App<'static> {
    let path_replacement = App::new("path_replacement")
        .about("Replaces all env. paths in the books")
        .subcommand(
            App::new("supports")
                .arg(Arg::new("renderer").required(true))
                .about("Check whether a renderer is supported by this preprocessor"),
        );

    let auto_doc = App::new("auto_doc")
        .about("Will auto replace all `tm_type` with links to doc")
        .subcommand(
            App::new("supports")
                .arg(Arg::new("renderer").required(true))
                .about("Check whether a renderer is supported by this preprocessor"),
        );

    let auto_include = App::new("auto_include")
        .about("Will auto include code sippets")
        .subcommand(
            App::new("supports")
                .arg(Arg::new("renderer").required(true))
                .about("Check whether a renderer is supported by this preprocessor"),
        );

    let authors = App::new("authors")
        .about("Will add all contributers to the pages")
        .subcommand(
            App::new("supports")
                .arg(Arg::new("renderer").required(true))
                .about("Check whether a renderer is supported by this preprocessor"),
        );

    App::new("tmbook")
        .version("1.0")
        .author("Our Machinery")
        .subcommand(
            App::new("init")
                .about("Downloads the book repo if not present")
                .arg(Arg::new("path").required(false)),
        )
        .subcommand(
            App::new("serve")
                .about("Call mdbook serve in the current folder")
                .arg(
                    Arg::new("book")
                        .required(false)
                        .possible_values(["the_machinery_book", "tutorials"]),
                ),
        )
        .subcommand(
            App::new("build")
                .about("Call mdbook build in the current folder")
                .arg(
                    Arg::new("book")
                        .required(false)
                        .possible_values(["the_machinery_book", "tutorials"]),
                ),
        )
        .arg(
            Arg::new("bin-path")
                .long("bin-path")
                .takes_value(true)
                .help("Ensures the right folder for the binaries"),
        )
        .subcommand(path_replacement)
        .subcommand(auto_doc)
        .subcommand(auto_include)
        .subcommand(authors)
}

fn find_book(book: &String, current_dir: &Path) -> Option<PathBuf> {
    for entry in fs::read_dir(current_dir).unwrap() {
        if entry.is_ok() {
            let e = entry.unwrap();
            let path = e.path();
            if path.as_path().file_name().unwrap() == book.as_str() {
                return Some(path);
            }
            if e.file_type().unwrap().is_dir() {
                let res = find_book(book, path.as_path());
                if res.is_some() {
                    println!("{:?}", res);
                    return res;
                }
            }
        } else {
            continue;
        }
    }
    return None;
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let matches = make_app().get_matches();

    let alt_path = matches.value_of("bin-path");

    let bin_dir = get_bin_dir(alt_path);
    {
        let zip_name = Path::new(MDBOOK_URL).file_name().unwrap();
        let fname = bin_dir.join(Path::new(zip_name));
        if !fname.exists() {
            println!("Download mdbook...");
            fetch_url(MDBOOK_URL.to_string(), &fname).await.unwrap();
            unzip(&fname, &bin_dir).unwrap();
        }
    }
    {
        let zip_name = Path::new(CLANGFORMAT_URL).file_name().unwrap();
        let fname = bin_dir.join(Path::new(zip_name));
        if !fname.exists() {
            println!("Download clang format...");
            fetch_url(CLANGFORMAT_URL.to_string(), &fname)
                .await
                .unwrap();
            unzip(&fname, &bin_dir).unwrap();
        }
    }
    {
        let zip_name = Path::new(MDBOOK_TOC_URL).file_name().unwrap();
        let fname = bin_dir.join(Path::new(zip_name));
        if !fname.exists() {
            println!("Download mdbook-toc...");
            fetch_url(MDBOOK_TOC_URL.to_string(), &fname).await.unwrap();
            unzip(&fname, &bin_dir).unwrap();
        }
    }
    {
        let zip_name = Path::new(MDBOOK_LINKCHECK_URL).file_name().unwrap();
        let fname = bin_dir.join(Path::new(zip_name));
        if !fname.exists() {
            println!("Download mdbook-linkcheck...");
            fetch_url(MDBOOK_LINKCHECK_URL.to_string(), &fname)
                .await
                .unwrap();
            unzip(&fname, &bin_dir).unwrap();
        }
    }

    {
        let path = std::env::var("TM_BOOK_CODE_SNIPPETS");
        if path.is_err() {
            println!("Warning: Could not find: `TM_BOOK_CODE_SNIPPETS` trying to set the var automatically");
            let path = Path::new("./code_snippets");
            if path.exists() {
                std::env::set_var(
                    "TM_BOOK_CODE_SNIPPETS",
                    std::fs::canonicalize(&path).unwrap().as_os_str(),
                );
                println!(
                    "TM_BOOK_CODE_SNIPPETS: {:?}",
                    std::fs::canonicalize(&path).unwrap().as_os_str()
                );
            } else {
                let url = TM_BOOK_CODE_SNIPPETS;
                match Repository::clone(url, "./code_snippets") {
                    Ok(_) => {
                        std::env::set_var(
                            "TM_BOOK_CODE_SNIPPETS",
                            std::fs::canonicalize(&path).unwrap().as_os_str(),
                        );
                        println!(
                            "TM_BOOK_CODE_SNIPPETS: {:?}",
                            std::fs::canonicalize(&path).unwrap().as_os_str()
                        );
                    }
                    Err(_) => {
                        eprintln!("Cannot clone: {}", url);
                    }
                };
            }
        } else {
            //println!("Found: `TM_BOOK_CODE_SNIPPETS`: {:?}", path.unwrap());
        }
    }

    if let Some(sub_matches) = matches.subcommand_matches("authors") {
        if let Some(sub_args) = sub_matches.subcommand_matches("supports") {
            handle_supports(PreType::ReplacePaths, sub_args);
        } else if let Err(e) = handle_preprocessing(PreType::Authors) {
            eprintln!("{}", e);
            process::exit(1);
        }
    }

    if let Some(sub_matches) = matches.subcommand_matches("path_replacement") {
        if let Some(sub_args) = sub_matches.subcommand_matches("supports") {
            handle_supports(PreType::ReplacePaths, sub_args);
        } else if let Err(e) = handle_preprocessing(PreType::ReplacePaths) {
            eprintln!("{}", e);
            process::exit(1);
        }
    }

    if let Some(sub_matches) = matches.subcommand_matches("auto_doc") {
        if let Some(sub_args) = sub_matches.subcommand_matches("supports") {
            handle_supports(PreType::AutoDoc, sub_args);
        } else if let Err(e) = handle_preprocessing(PreType::AutoDoc) {
            eprintln!("{}", e);
            process::exit(1);
        }
    }

    if let Some(sub_matches) = matches.subcommand_matches("auto_include") {
        if let Some(sub_args) = sub_matches.subcommand_matches("supports") {
            handle_supports(PreType::AutoInclude, sub_args);
        } else if let Err(e) = handle_preprocessing(PreType::AutoInclude) {
            eprintln!("{}", e);
            process::exit(1);
        }
    }

    if let Some(sub_args) = matches.subcommand_matches("init") {
        let var = sub_args.value_of("path");
        let path = if var.is_some() {
            var.unwrap()
        } else {
            "./the_machinery_book"
        };

        println!("Download The Machinery Book Repo to `{:?}` ...", path);

        let url = TM_BOOKS_REPO;
        match Repository::clone(url, path) {
            Ok(_) => {
                println!("The book is downloaded and ready!");
                std::env::set_current_dir(Path::new(path).join("the_machinery_book"))
                    .expect("Could not find the folder");
            }
            Err(e) => {
                eprintln!("Could not download the book: {}", e.message());
                process::exit(1);
            }
        };
    }

    if let Some(sub_args) = matches.subcommand_matches("serve") {
        let cwd = std::env::current_dir().unwrap();
        let mdbook_bin = cwd.join(bin_dir.join("mdbook.exe"));
        let var = sub_args.value_of("book");
        if var.is_some() {
            let var = var.unwrap().to_string();
            let current_dir = find_book(&var, &cwd);
            if current_dir.is_some() {
                std::env::set_current_dir(current_dir.unwrap()).expect("Could not find the folder");
            } else {
                std::env::set_current_dir(std::env::current_dir().unwrap())
                    .expect("Could not find the folder");
            }
        }
        let mut child = Command::new(mdbook_bin)
            .arg("serve")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()
            .expect("failed to run command");
        let stdout = child
            .stdout
            .take()
            .expect("child did not have a handle to stdout");
        let mut reader = BufReader::new(stdout).lines();
        tokio::spawn(async move {
            let status = child
                .wait()
                .await
                .expect("child process encountered an error");

            println!("child status was: {}", status);
        });
        while let Some(line) = reader.next_line().await? {
            println!("{}", line);
        }
    }
    if let Some(sub_args) = matches.subcommand_matches("build") {
        let cwd = std::env::current_dir().unwrap();
        let mdbook_bin = cwd.join(bin_dir.join("mdbook.exe"));
        let var = sub_args.value_of("book");
        if var.is_some() {
            let var = var.unwrap().to_string();
            let current_dir = find_book(&var, &cwd);
            if current_dir.is_some() {
                std::env::set_current_dir(current_dir.unwrap()).expect("Could not find the folder");
            } else {
                std::env::set_current_dir(std::env::current_dir().unwrap())
                    .expect("Could not find the folder");
            }
        }
        let mut child = Command::new(mdbook_bin)
            .arg("build")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()
            .expect("failed to run command");
        let stdout = child
            .stdout
            .take()
            .expect("child did not have a handle to stdout");
        let mut reader = BufReader::new(stdout).lines();
        tokio::spawn(async move {
            let status = child
                .wait()
                .await
                .expect("child process encountered an error");

            println!("child status was: {}", status);
        });
        while let Some(line) = reader.next_line().await? {
            println!("{}", line);
        }
    }

    Ok(())
}

fn handle_preprocessing(pre: PreType) -> Result<(), Error> {
    let (ctx, book) = CmdPreprocessor::parse_input(io::stdin())?;

    if pre == PreType::ReplacePaths {
        let processed_book = ReplacePaths.run(&ctx, book)?;
        serde_json::to_writer(io::stdout(), &processed_book)?;
    } else if pre == PreType::AutoDoc {
        let processed_book = AutoDoc.run(&ctx, book)?;
        serde_json::to_writer(io::stdout(), &processed_book)?;
    } else if pre == PreType::AutoInclude {
        let processed_book = AutoInclude.run(&ctx, book)?;
        serde_json::to_writer(io::stdout(), &processed_book)?;
    } else if pre == PreType::Authors {
        let processed_book = Authors.run(&ctx, book)?;
        serde_json::to_writer(io::stdout(), &processed_book)?;
    }

    Ok(())
}

fn handle_supports(pre: PreType, sub_args: &ArgMatches) -> ! {
    let renderer = sub_args.value_of("renderer").expect("Required argument");
    let supported;

    if pre == PreType::ReplacePaths {
        supported = ReplacePaths.supports_renderer(renderer);
    } else if pre == PreType::AutoDoc {
        supported = AutoDoc.supports_renderer(renderer);
    } else if pre == PreType::AutoInclude {
        supported = AutoInclude.supports_renderer(renderer);
    } else {
        supported = Authors.supports_renderer(renderer);
    }

    // Signal whether the renderer is supported by exiting with 1 or 0.
    if supported {
        process::exit(0);
    } else {
        process::exit(1);
    }
}
