extern crate mdbook;
extern crate serde_json;

use mdbook::book::{Book, BookItem};
use mdbook::errors::Result;
use mdbook::preprocess::{Preprocessor, PreprocessorContext};
use pulldown_cmark::{CowStr, Event, Options, Parser, Tag};
use pulldown_cmark_to_cmark::cmark;
use regex::Regex;
use std::env;
use std::io::{self, BufRead, Write};
use std::path::Path;
use std::process::{Command, Stdio};

use crate::utility::get_bin_dir;
pub struct AutoInclude;

fn read_lines<P>(filename: P) -> io::Result<io::Lines<io::BufReader<std::fs::File>>>
where
    P: AsRef<Path>,
{
    let file = std::fs::File::open(filename)?;
    Ok(io::BufReader::new(file).lines())
}

#[derive(Debug)]
enum Token {
    CodeSnippetBegin(Option<String>),
    CodeSnippetEnd(Option<String>),
    CodeSnippetExcludeEnd(Option<String>),
    CodeSnippetExcludeBegin(Option<String>),
}

fn get_tag(source: &String, token: &str) -> Option<String> {
    let regex = format!(r"#{}\((.+)\)", token);
    let re = Regex::new(&regex).unwrap();
    let res = re.captures_iter(&source);
    for cap in res {
        return Some(cap[1].to_string());
    }
    None
}

fn extract_token<'a>(source: &String) -> Option<Token> {
    if source.contains("#code_snippet_exclude_begin()") {
        return Some(Token::CodeSnippetExcludeBegin(None));
    } else if source.contains("#code_snippet_exclude_end()") {
        return Some(Token::CodeSnippetExcludeEnd(None));
    } else if source.contains("#code_snippet_exclude_begin(") {
        let tag = get_tag(source, "code_snippet_exclude_begin");
        return Some(Token::CodeSnippetExcludeBegin(tag));
    } else if source.contains("#code_snippet_exclude_end(") {
        let tag = get_tag(source, "code_snippet_exclude_end");
        return Some(Token::CodeSnippetExcludeEnd(tag));
    } else if source.contains("#code_snippet_begin(") {
        let tag = get_tag(source, "code_snippet_begin");
        return Some(Token::CodeSnippetBegin(tag));
    } else if source.contains("#code_snippet_end(") {
        let tag = get_tag(source, "code_snippet_end");
        return Some(Token::CodeSnippetEnd(tag));
    }
    None
}

fn replace_env(path: String) -> String {
    let regex = r"(env\.([A-Z_0-9]+))";
    let re = Regex::new(regex).unwrap();
    let cap = re.captures(&path);
    let mut res = String::from(&path);
    if cap.is_some() {
        let cap = cap.unwrap();
        let value = env::var(&cap[2]);
        res = path.replace(&cap[1], value.unwrap().as_str())
    }
    res
}

fn process_term(
    source: io::Lines<io::BufReader<std::fs::File>>,
    requested_tag: &String,
    ignore_exclude: bool,
) -> Vec<String> {
    let mut tag_content = Vec::<String>::new();
    let mut tag_exclude = false;
    let mut current_tag = None;
    for line in source {
        let line = line.unwrap();
        let token = extract_token(&line);
        if token.is_some() {
            let token = token.unwrap();
            match token {
                Token::CodeSnippetBegin(tag) => {
                    if requested_tag == &tag.unwrap() {
                        //println!("open {:?}", requested_tag);
                        current_tag = Some(String::from(requested_tag));
                        tag_exclude = false;
                    }
                    continue;
                }
                Token::CodeSnippetEnd(tag) => {
                    if requested_tag == &tag.unwrap() {
                        // println!("close {:?}", requested_tag);
                        current_tag = None;
                    }
                    continue;
                }
                Token::CodeSnippetExcludeBegin(tag) => {
                    tag_exclude = if tag.is_none() {
                        true
                    } else if requested_tag == &tag.unwrap() {
                        true
                    } else {
                        tag_exclude
                    };
                    if tag_exclude && ignore_exclude {
                        tag_exclude = false;
                    }
                    continue;
                }
                Token::CodeSnippetExcludeEnd(tag) => {
                    tag_exclude = if tag.is_none() {
                        false
                    } else if requested_tag == &tag.unwrap() {
                        false
                    } else {
                        tag_exclude
                    };
                    continue;
                }
            }
        }
        match &current_tag {
            Some(tag) => {
                if tag == requested_tag && !tag_exclude {
                    tag_content.push(String::from(&line));
                }
            }
            _ => {}
        }
    }
    tag_content
}

fn process_clang_format(source: String) -> String {
    // TODO: fix the hardcoded path!
    let bin_dir = get_bin_dir(None)
        .join("clang-format-6.0.0-win64")
        .join("clang-format-6.0.exe");
    let mut child = Command::new(bin_dir)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .expect("cannot call clang format");

    let child_stdin = child.stdin.as_mut().unwrap();
    child_stdin.write_all(source.as_bytes()).unwrap();
    drop(child_stdin);
    let output = child.wait_with_output().unwrap();

    String::from_utf8(output.stdout).unwrap()
}

fn find_term(mut chapter: String) -> String {
    let mut regex = r"\{\{insert_code\(([/aA-zZ_.0-9]+),([/aA-zZ_0-9]+)\)\}\}";
    let mut re = Regex::new(regex).unwrap();
    let str = chapter.clone();
    let res = if re.is_match(&str) {
        re.captures_iter(&str)
    } else {
        regex = r"\{\{insert_code\(([/aA-zZ_.0-9]+),([/aA-zZ_0-9]+),([aA-zZ_0-9]+)\)\}\}";
        re = Regex::new(regex).unwrap();
        re.captures_iter(&str)
    };
    if re.is_match(&str) {
        for cap in res {
            let requested_tag = cap[2].to_string();
            let path = replace_env(cap[1].to_string());
            let path = Path::new(&path);
            if path.exists() {
                if let Ok(lines) = read_lines(path) {
                    let content = process_term(lines, &requested_tag, cap.len() == 5);
                    let content = process_clang_format(content.join("\n"));
                    chapter = chapter.replace(&cap[0], content.as_str());
                }
            } else {
                eprint!("Path: {:?} does not exist!", path);
            }
        }
    } else {
        regex = r"\{\{insert_code\(([/aA-zZ_.0-9]+)\)\}\}";
        let nre = Regex::new(regex).unwrap();
        let results = nre.captures_iter(&str);
        for cap in results {
            let mut tag_content = Vec::<String>::new();
            let path = replace_env(cap[1].to_string());
            let path = Path::new(&path);
            if path.exists() {
                if let Ok(lines) = read_lines(path) {
                    for line in lines {
                        let line = line.unwrap();
                        let token = extract_token(&line);
                        if token.is_none() {
                            tag_content.push(line);
                        }
                    }
                }
                let content = tag_content.join("\n");
                chapter = chapter.replace(&cap[0], content.as_str());
            }
        }
    }

    return chapter;
}

pub fn process(chapter: String) -> Option<String> {
    let mut opts = Options::empty();
    opts.insert(Options::ENABLE_TABLES);
    opts.insert(Options::ENABLE_FOOTNOTES);
    opts.insert(Options::ENABLE_STRIKETHROUGH);
    opts.insert(Options::ENABLE_TASKLISTS);

    let iter = Parser::new_ext(&chapter, opts).into_iter();
    let mut events = Vec::<Event>::new();
    let mut found_code_block = false;
    for i in iter {
        match i {
            Event::Start(Tag::CodeBlock(kind)) => {
                found_code_block = true;
                events.push(Event::Start(Tag::CodeBlock(kind)));
            }
            Event::End(Tag::CodeBlock(kind)) => {
                found_code_block = false;
                events.push(Event::End(Tag::CodeBlock(kind)));
            }
            Event::Text(mut text) => {
                if found_code_block {
                    text = CowStr::Boxed(find_term(text.to_string()).into_boxed_str());
                }
                events.push(Event::Text(text));
            }
            _ => {
                events.push(i);
            }
        }
    }

    let mut buf = String::new();
    cmark(events.into_iter(), &mut buf).unwrap();

    Some(buf)
}

impl Preprocessor for AutoInclude {
    fn name(&self) -> &str {
        "auto_include"
    }

    fn run(&self, _: &PreprocessorContext, mut book: Book) -> Result<Book> {
        let res = None;
        book.for_each_mut(|item: &mut BookItem| {
            if let Some(Err(_)) = res {
                return;
            }

            if let BookItem::Chapter(ref mut chapter) = *item {
                let content = chapter.content.to_string();
                chapter.content = process(content).unwrap();
            }
        });

        res.unwrap_or(Ok(())).map(|_| book)
    }

    fn supports_renderer(&self, _renderer: &str) -> bool {
        true
    }
}
