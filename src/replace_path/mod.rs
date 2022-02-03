extern crate mdbook;
extern crate serde_json;

use mdbook::book::{Book, BookItem};
use mdbook::errors::Result;
use mdbook::preprocess::{Preprocessor, PreprocessorContext};
use regex::Regex;
//use std::convert::TryInto;
use std::fs;
//use toml::value::Table;

pub struct ReplacePaths;

pub fn load_config() -> serde_json::Value {
    let path = "./context.json";
    let data = fs::read_to_string(path).expect("Unable to read file");
    serde_json::from_str(&data).expect("Unable to parse")
}

fn find_term(mut chapter: String) -> Option<String> {
    let config = load_config();
    let object = config.as_object().unwrap();
    let re = Regex::new(r"(\{\{([a-z_]+)\}\})").unwrap();
    let str = chapter.clone();
    let res = re.captures_iter(&str);
    for cap in res {
        if object.contains_key(&cap[2]) {
            let (term, repl) = object.get_key_value(&cap[2]).unwrap();
            let a = r"(\{\{";
            let b = r"\}\})";
            let r = [a, term.as_str(), b].join("");
            let regex = Regex::new(&r).unwrap();
            let rstr = repl.as_str().unwrap();
            chapter = regex.replace_all(chapter.as_str(), rstr).to_string();
        }
    }
    Some(chapter)
}

impl Preprocessor for ReplacePaths {
    fn name(&self) -> &str {
        "replace_paths"
    }

    fn run(&self, _: &PreprocessorContext, mut book: Book) -> Result<Book> {
        let res = None;

        book.for_each_mut(|item: &mut BookItem| {
            if let Some(Err(_)) = res {
                return;
            }

            if let BookItem::Chapter(ref mut chapter) = *item {
                let content = chapter.content.to_string();
                chapter.content = find_term(content).unwrap();
            }
        });

        res.unwrap_or(Ok(())).map(|_| book)
    }

    fn supports_renderer(&self, _renderer: &str) -> bool {
        true
    }
}
