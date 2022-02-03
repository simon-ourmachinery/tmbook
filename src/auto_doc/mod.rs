extern crate mdbook;
extern crate serde_json;

use mdbook::book::{Book, BookItem};
use mdbook::errors::Result;
use mdbook::preprocess::{Preprocessor, PreprocessorContext};
use pulldown_cmark::{CowStr, Event, LinkType, Options, Parser, Tag};
use pulldown_cmark_to_cmark::cmark;
use regex::Regex;
use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use std::fs;

#[derive(Serialize, Deserialize, Debug)]
pub struct Term {
    term: String,
    path: String,
    file: String,
    name: String,
    id: serde_json::Number,
}

pub struct AutoDoc;

pub fn load_config() -> HashMap<String, Term> {
    let path = "../terms.json";
    let data = fs::read_to_string(path).expect("Unable to read file");
    let terms: Vec<Term> = serde_json::from_str(&data).expect("Unable to parse");
    let mut res = HashMap::<String, Term>::new();
    res.reserve(terms.len());

    for term in terms {
        res.insert(String::from(term.term.as_str()), term);
    }

    res
}

pub fn process(lookup: &HashMap<String, Term>, chapter: String) -> Option<String> {
    let mut opts = Options::empty();
    opts.insert(Options::ENABLE_TABLES);
    opts.insert(Options::ENABLE_FOOTNOTES);
    opts.insert(Options::ENABLE_STRIKETHROUGH);
    opts.insert(Options::ENABLE_TASKLISTS);

    let iter = Parser::new_ext(chapter.as_str(), opts).into_iter();
    let mut events = Vec::<Event>::new();
    for i in iter {
        match i {
            Event::Code(text) => {
                let key = text.to_string().replace("\"", "");
                let re = Regex::new(r"\(.*\)").unwrap();
                let key = key.replace(r"", "");
                let key = re.replace_all(&key, "()").to_string();
                let alt_key = format!("{}()", key);
                if lookup.contains_key(&key) || lookup.contains_key(&alt_key) {
                    let term = lookup.get(&key);
                    let term = if term.is_none() {
                        lookup.get(&alt_key).unwrap()
                    } else {
                        term.unwrap()
                    };

                    let link = format!("{}{}.html#{}", "{{docs}}", term.path, term.name);
                    let b = link.into_boxed_str();
                    let e = Event::Start(Tag::Link(
                        LinkType::Inline,
                        text.clone(),
                        CowStr::Borrowed(""),
                    ));
                    events.push(e);
                    let e = Event::Code(text.clone());
                    events.push(e);
                    let e = Event::End(Tag::Link(
                        LinkType::Inline,
                        CowStr::Boxed(b),
                        CowStr::Borrowed(""),
                    ));
                    events.push(e);
                } else {
                    // TODO: enable verbose
                    //eprint!("Warning: Unknown tag: `{}`\n", text);
                    let e = Event::Code(text.clone());
                    events.push(e);
                }
            }
            _ => {
                events.push(i);
            }
        };
    }

    let mut buf = String::new();
    cmark(events.into_iter(), &mut buf).unwrap();

    Some(buf)
}

impl Preprocessor for AutoDoc {
    fn name(&self) -> &str {
        "auto_doc"
    }

    fn run(&self, _: &PreprocessorContext, mut book: Book) -> Result<Book> {
        let res = None;
        let lookup = load_config();
        book.for_each_mut(|item: &mut BookItem| {
            if let Some(Err(_)) = res {
                return;
            }

            if let BookItem::Chapter(ref mut chapter) = *item {
                let content = chapter.content.to_string();
                chapter.content = process(&lookup, content).unwrap();
            }
        });

        res.unwrap_or(Ok(())).map(|_| book)
    }

    fn supports_renderer(&self, _renderer: &str) -> bool {
        true
    }
}
