use std::collections::HashMap;

use mdbook::errors::Result;
use mdbook::{
    book::Book,
    preprocess::{Preprocessor, PreprocessorContext},
    BookItem,
};

use git2::Repository;

pub struct Authors;

struct Entry {
    number: i64,
    name: String,
}

// https://github.com/rust-lang/git2-rs/blob/master/examples/log.rs
macro_rules! filter_try {
    ($e:expr) => {
        match $e {
            Ok(t) => t,
            Err(e) => return Some(Err(e)),
        }
    };
}

pub fn process(
    repo: &Result<Repository, git2::Error>,
    file: &std::path::Path,
    source: &String,
) -> Option<String> {
    let mut res = None;
    match repo {
        Ok(repo) => {
            let mut revwalk = repo.revwalk().unwrap();
            revwalk
                .set_sorting(git2::Sort::REVERSE | git2::Sort::TOPOLOGICAL | git2::Sort::TIME)
                .unwrap();
            revwalk.push_head().unwrap();
            let revwalk = revwalk.map(|id| {
                let id = filter_try!(id);
                let commit = filter_try!(repo.find_commit(id));
                Some(Ok(commit))
            });
            let mut contributors = HashMap::<String, Entry>::new();
            for commit in revwalk {
                let commit = commit.unwrap();
                let commit = commit.unwrap();
                let tree = commit.tree().unwrap();
                let entry = tree.get_path(file);
                if entry.is_ok() {
                    let email = commit.author().email().unwrap().to_string();
                    let name = commit.author().name().unwrap().to_string();
                    if !contributors.contains_key(&email)
                        && !email.contains("users.noreply.github.com")
                    {
                        contributors.insert(
                            email,
                            Entry {
                                number: 1,
                                name: name,
                            },
                        );
                    } else if !email.contains("users.noreply.github.com") {
                        let mut val = contributors.get_mut(&email).unwrap();
                        val.number += 1;
                    }
                } else {
                    //eprintln!("{:?}", file.as_os_str());
                }
            }
            let mut sorted: Vec<_> = contributors.iter().collect();
            sorted.sort_by(|a, b| b.1.number.cmp(&a.1.number));

            if sorted.len() > 0 {
                let mut str = format!("{}\n# Contributors\n", source.as_str());
                for (email, entry) in sorted {
                    str = format!(
                        "{}\n[![{}](https://www.gravatar.com/avatar/{}?s=32) {}](mailto:{})",
                        str.clone(),
                        &entry.name,
                        &email,
                        &email,
                        &email
                    );
                }
                res = Some(str);
            }
        }
        Err(_) => return None,
    };
    return res;
}

impl Preprocessor for Authors {
    fn name(&self) -> &str {
        "authors"
    }

    fn run(&self, _: &PreprocessorContext, mut book: Book) -> Result<Book> {
        let res = None;
        book.for_each_mut(|item: &mut BookItem| {
            if let Some(Err(_)) = res {
                return;
            }

            if let BookItem::Chapter(ref mut chapter) = *item {
                let content = chapter.content.to_string();
                let path = chapter.source_path.as_ref();
                let cwd = std::env::current_dir().unwrap();
                let repo = Repository::open(&cwd);

                let file =
                    std::path::Path::new(std::env::current_dir().unwrap().file_name().unwrap())
                        .join("src")
                        .join(path.unwrap());
                let processed_data = process(&repo, &file, &content);
                if processed_data.is_none() {
                    let file_path = std::path::Path::new(
                        cwd.parent().unwrap().file_name().unwrap().to_str().unwrap(),
                    )
                    .join("src")
                    .join(path.unwrap());
                    let repo = Repository::open(cwd.parent().unwrap());
                    let processed_data = process(&repo, &file_path, &content);
                    if processed_data.is_some() {
                        chapter.content = processed_data.unwrap();
                    } else {
                        let processed_data = process(&repo, &file_path, &content);
                        if processed_data.is_some() {
                            chapter.content = processed_data.unwrap();
                        }
                    }
                } else {
                    chapter.content = processed_data.unwrap();
                }
            }
        });

        res.unwrap_or(Ok(())).map(|_| book)
    }

    fn supports_renderer(&self, _renderer: &str) -> bool {
        true
    }
}
