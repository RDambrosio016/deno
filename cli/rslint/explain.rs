//! CLI rule explanations for `deno lint explain <rules>` utilizing `rslint_lexer` for
//! token-based highlighting.

use crate::http_util::{create_http_client, fetch_once, FetchOnceResult};
use deno_core::error::generic_error;
use deno_core::url::Url;
use regex::{Captures, Regex};
use rslint_lexer::{
  ansi_term::{self, Color::*},
  color,
};

const DOCS_LINK_BASE: &str =
  "https://raw.githubusercontent.com/RDambrosio016/RSLint/master/docs/rules";
const WEBSITE_DOCS_BASE: &str = "https://rdambrosio016.github.io/RSLint/rules";

/// A structure for converting user facing markdown docs to ANSI colored terminal explanations.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ExplanationRunner {
  pub rules: Vec<String>,
  pub rule_names: Vec<String>,
}

impl ExplanationRunner {
  /// Make a new runner and try to fetch the remote docs files for each rule.
  /// This automatically issues any linter errors for invalid rules.
  pub async fn new(rules: Vec<String>) -> Self {
    let mut docs = Vec::with_capacity(rules.len());
    for rule in &rules {
      let res = fetch_doc_file(&rule).await;
      if let Some(doc) = res {
        docs.push(doc);
      } else {
        eprintln!(
          "{}",
          generic_error(format!("Failed to fetch rule docs for `{}`", rule))
            .to_string()
        );
      }
    }

    Self {
      rules: docs,
      rule_names: rules,
    }
  }

  pub fn strip_rule_preludes(&mut self) {
    for rule in self.rules.iter_mut() {
      rule.replace_range(0..70, "");
    }
  }

  pub fn replace_headers(&mut self) {
    let regex = Regex::new("#+ (.*)").unwrap();
    for rule in self.rules.iter_mut() {
      *rule = regex
        .replace_all(rule, |cap: &Captures| {
          White.bold().paint(cap.get(1).unwrap().as_str()).to_string()
        })
        .to_string();
    }
  }

  pub fn replace_code_blocks(&mut self) {
    let regex = Regex::new("```js\n([\\s\\S]*?)\n```").unwrap();
    for rule in self.rules.iter_mut() {
      *rule = regex
        .replace_all(rule, |cap: &Captures| {
          format!("\n{}\n", color(cap.get(1).unwrap().as_str()))
        })
        .to_string();
    }
  }

  pub fn strip_config_or_extra_examples(&mut self) {
    for rule in self.rules.iter_mut() {
      if let Some(idx) = rule.find("# Config") {
        rule.truncate(idx - 1);
      }
      if let Some(idx) = rule.find("<details>") {
        rule.truncate(idx - 1);
      }
    }
  }

  pub fn replace_inline_code_blocks(&mut self) {
    let regex = Regex::new("`(.+?)`").unwrap();
    for rule in self.rules.iter_mut() {
      *rule = regex
        .replace_all(rule, |cap: &Captures| {
          let color = RGB(42, 42, 42);
          ansi_term::Style::new()
            .on(color)
            .fg(White)
            .paint(cap.get(1).unwrap().as_str())
            .to_string()
        })
        .to_string();
    }
  }

  pub fn append_link_to_docs(&mut self) {
    for (docs, name) in self.rules.iter_mut().zip(self.rule_names.iter()) {
      let group = rslint_core::get_rule_by_name(&name).unwrap().group();
      let link = format!("{}/{}/{}.md", WEBSITE_DOCS_BASE, group, name);
      docs.push_str(&format!(
        "{}: {}\n",
        Green.paint("Docs").to_string(),
        link
      ));
    }
  }

  pub fn render(&mut self) {
    self.strip_rule_preludes();
    self.strip_config_or_extra_examples();
    self.replace_headers();
    self.replace_code_blocks();
    self.replace_inline_code_blocks();
    self.append_link_to_docs();
  }

  pub fn print(mut self) {
    self.render();
    for rule in self.rules.into_iter() {
      println!("{}", "-".repeat(10));
      println!("{}", rule);
    }
  }
}

/// Try to resolve a rule name, then fetch its remote docs file.
pub(crate) async fn fetch_doc_file(rule: &str) -> Option<String> {
  let resolved_rule = rslint_core::get_rule_by_name(rule)?;
  let url = format!("{}/{}/{}.md", DOCS_LINK_BASE, resolved_rule.group(), rule);
  let client = create_http_client(None).ok()?;
  let res = fetch_once(client, &url.parse::<Url>().unwrap(), None)
    .await
    .ok()?;
  match res {
    FetchOnceResult::Code(data, _) => {
      Some(String::from_utf8(data).expect("Unexpected non-utf8 rule docs"))
    }
    // FIXME(RDambrosio016): should this panic or throw an error?
    FetchOnceResult::Redirect(_, _) => {
      panic!("Unexpected redirect while fetching rule docs")
    }
    FetchOnceResult::NotModified => {
      panic!("Unexpected 304(Not modified) result")
    }
  }
}
