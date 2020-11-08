extern crate lalrpop;
extern crate regex;

use regex::Regex;
use std::collections::HashSet;
use std::env;
use std::fs::File;
use std::io::{prelude::*, BufReader};
use std::path::Path;

fn main() {
    extract_token_names("src/parser.lalrpop").unwrap();
    lalrpop::Configuration::new()
        .emit_rerun_directives(true)
        .process_current_dir()
        .unwrap();
}

// Token name extraction

fn starts_ident(c: char) -> bool {
    c.is_ascii_alphabetic() || c == '.' || c == '_'
}

fn is_ident_char(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '#' || c == '.' || c == '@' || c == '_'
}

fn to_index(c: char) -> usize {
    (if c.is_ascii_lowercase() {
        c.to_ascii_uppercase()
    } else {
        c
    }) as usize
        - '#' as usize
}

#[derive(Default)]
struct TrieNode {
    children: Vec<usize>,
    mapping: Option<String>,
}

struct KeywordTrie {
    nodes: Vec<TrieNode>,
}

impl KeywordTrie {
    fn new() -> Self {
        Self {
            nodes: vec![Default::default()],
        }
    }

    fn add(&mut self, name: &str, val: String) {
        // TODO: avoid duplicating this code with `src/lexer.rs`
        assert!(
            starts_ident(
                name.chars()
                    .nth(0)
                    .expect("Keyword names shouldn't be empty")
            ),
            "{} does not begin correctly (A-Za-z._)",
            name
        );

        let mut i: usize = 0;
        for c in name.chars() {
            assert!(
                is_ident_char(c),
                "{} does not contain only [0-9A-Z-a-z#.@_] ({})",
                name,
                c
            );

            let index = to_index(c);
            if self.nodes[i].children.len() <= index {
                self.nodes[i].children.resize(index + 1, 0);
            }
            // If no child node there, allocate a new one
            if self.nodes[i].children[index] == 0 {
                let nb_nodes = self.nodes.len();
                self.nodes.resize_with(nb_nodes + 1, Default::default);
                self.nodes[i].children[index] = nb_nodes;
            }
            i = self.nodes[i].children[index];
        }
        self.nodes[i].mapping = Some(val);
    }

    // Output functions

    fn write(&self, f: &mut File) {
        let mut seen: HashSet<usize> = HashSet::new();

        self.write_node(0, f, &mut seen);

        // Check that all nodes have been output
        for i in 0..self.nodes.len() {
            assert!(seen.contains(&i), "Keyword node {} not seen!?", i);
        }
    }

    fn write_node(&self, i: usize, f: &mut File, seen: &mut HashSet<usize>) {
        assert!(seen.insert(i)); // We should never be writing the same node twice

        write!(
            f,
            "TrieNode {{
    value: {},
    children: &[
",
            match &self.nodes[i].mapping {
                Some(string) => format!("Some({})", string),
                None => "None".to_string(),
            }
        )
        .unwrap();
        for &child in &self.nodes[i].children {
            if child == 0 {
                write!(f, "None,\n").unwrap();
            } else {
                write!(f, "Some(\n").unwrap();
                self.write_node(child, f, seen);
                write!(f, "),\n").unwrap();
            }
        }
        write!(f, "],\n}}\n").unwrap();
    }
}

fn extract_token_names(parser_file_name: &str) -> std::io::Result<()> {
    println!("{}", format!("cargo:rerun-if-changed={}", parser_file_name));
    let out_dir = env::var("OUT_DIR").unwrap();

    let mut tok_names = File::create(Path::new(&out_dir).join("token_names.rs"))?;
    let parser_file = File::open(parser_file_name)?;

    let regex = Regex::new("^\\s*\"?(.+?)\"?\\s*=>\\s*lexer::(.+?)((\\(.+)\\))?,").unwrap();
    let mut reg_locs = regex.capture_locations();

    // Prologue
    tok_names.write_all(
        b"impl Display for TokType {
    fn fmt(&self, fmt: &mut Formatter) -> Result<(), fmt::Error> {
    	write!(fmt, \"{}\", match self {
",
    )?;

    // Body
    let mut reading_keywords = true;
    let mut keywords = KeywordTrie::new();

    for line in BufReader::new(parser_file)
        .lines()
        .skip_while(|line| {
            line.as_ref()
                .map_or(true, |line| !line.contains("enum lexer::TokType {"))
        })
        .skip(1) // Skip the `enum lexer::TokType {` line as well
        .take_while(|line| line.as_ref().map_or(true, |line| !line.contains('}')))
    {
        let line = line.unwrap();
        // Truncation OK because the source indexes originate from the string anyways
        let slice = |ends: &(usize, usize)| &line[ends.0..ends.1];

        tok_names
            .write_all(
                format!(
                    "    {}\n",
                    match regex.captures_read(&mut reg_locs, &line) {
                        Some(_) => {
                            let name = slice(&reg_locs.get(1).unwrap());
                            let mut val = slice(&reg_locs.get(2).unwrap()).to_string();
                            if let Some(_) = reg_locs.get(3) {
                                val.push_str("(_)");
                            }

                            let out = format!("        {} => \"{}\",", val, name);
                            if reading_keywords {
                                keywords.add(name, val)
                            }
                            out
                        }
                        None => {
                            reading_keywords &= line.is_empty();
                            line
                        }
                    }
                )
                .as_bytes(),
            )
            .unwrap();
    }

    // Epilogue
    tok_names.write_all(
        b"        })
	}
}",
    )?;

    // Write (optimized) keywords trie
    let mut keywords_file = File::create(Path::new(&out_dir).join("keywords.rs"))?;
    keywords_file
        .write_all(
            b"struct TrieNode {
    children: &'static [Option<TrieNode>],
    value: Option<TokType>,
}

static KEYWORDS: TrieNode =
",
        )
        .unwrap();
    keywords.write(&mut keywords_file);
    keywords_file.write_all(b";\n").unwrap();

    Ok(())
}
