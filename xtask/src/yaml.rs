//! A reader for the YAML subset an issue form is written in.
//!
//! Not a YAML implementation, and it says so rather than guessing: block mappings, block sequences,
//! plain and quoted scalars, and `|` block scalars. Flow collections, folded scalars, anchors, tags
//! and tab indentation are refused with the line that caused them. The job is not to accept any YAML
//! a person might write — it is to refuse a form GitHub would reject, and to fail on the line here
//! rather than in the browser of whoever opened the issue.

/// One value from the document.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Node {
    Scalar(String),
    List(Vec<Node>),
    Map(Vec<(String, Node)>),
}

impl Node {
    /// The value under `key`, when this node is a mapping.
    pub fn field(&self, key: &str) -> Option<&Node> {
        match self {
            Node::Map(entries) => entries.iter().find(|(name, _)| name == key).map(|(_, value)| value),
            _ => None,
        }
    }

    /// The text, when this node is a scalar.
    pub fn as_str(&self) -> Option<&str> {
        match self {
            Node::Scalar(text) => Some(text),
            _ => None,
        }
    }

    /// The items, when this node is a sequence.
    pub fn as_list(&self) -> Option<&[Node]> {
        match self {
            Node::List(items) => Some(items),
            _ => None,
        }
    }

    /// The entries, when this node is a mapping.
    pub fn as_map(&self) -> Option<&[(String, Node)]> {
        match self {
            Node::Map(entries) => Some(entries),
            _ => None,
        }
    }
}

/// Read a document, or name the line that stops it.
pub fn parse(source: &str) -> Result<Node, String> {
    let mut lines = Vec::new();
    for (index, raw) in source.lines().enumerate() {
        let number = index + 1;
        let indent = raw.len() - raw.trim_start_matches(' ').len();
        if raw[indent..].starts_with('\t') {
            return Err(format!("line {number}: a tab cannot indent YAML"));
        }
        lines.push(Line {
            number,
            indent,
            raw: raw.to_string(),
        });
    }
    Parser { lines, at: 0 }.document()
}

/// One source line: where it starts, and what it says after that.
struct Line {
    number: usize,
    indent: usize,
    raw: String,
}

impl Line {
    fn text(&self) -> &str {
        &self.raw[self.indent..]
    }

    fn blank(&self) -> bool {
        self.raw.trim().is_empty()
    }

    fn comment(&self) -> bool {
        self.text().starts_with('#')
    }
}

struct Parser {
    lines: Vec<Line>,
    at: usize,
}

impl Parser {
    fn document(mut self) -> Result<Node, String> {
        self.skip();
        let Some(line) = self.lines.get(self.at) else {
            return Err("the document is empty".to_string());
        };
        if line.indent != 0 {
            return Err(format!("line {}: the document starts indented", line.number));
        }
        let node = self.block(0)?;
        self.skip();
        if let Some(line) = self.lines.get(self.at) {
            return Err(format!("line {}: unexpected content after the document", line.number));
        }
        Ok(node)
    }

    fn skip(&mut self) {
        while self.lines.get(self.at).is_some_and(|line| line.blank() || line.comment()) {
            self.at += 1;
        }
    }

    fn block(&mut self, indent: usize) -> Result<Node, String> {
        self.skip();
        let Some(line) = self.lines.get(self.at) else {
            return Err("a block was expected, and the document ended".to_string());
        };
        if line.indent != indent {
            return Err(format!(
                "line {}: expected {indent} spaces of indentation, found {}",
                line.number, line.indent
            ));
        }
        let text = line.text().to_string();
        if text == "-" || text.starts_with("- ") {
            self.sequence(indent)
        } else {
            self.mapping(indent)
        }
    }

    fn mapping(&mut self, indent: usize) -> Result<Node, String> {
        let mut entries: Vec<(String, Node)> = Vec::new();
        loop {
            self.skip();
            let Some(line) = self.lines.get(self.at) else { break };
            if line.indent < indent {
                break;
            }
            if line.indent > indent {
                return Err(format!(
                    "line {}: {} spaces of indentation where {indent} or less were expected",
                    line.number, line.indent
                ));
            }
            let number = line.number;
            let text = line.text().to_string();
            if text == "-" || text.starts_with("- ") {
                return Err(format!("line {number}: a sequence entry where a `key: value` line was expected"));
            }
            let Some((key, rest)) = split_key(&text) else {
                return Err(format!("line {number}: `{text}` is not a `key: value` line"));
            };
            if entries.iter().any(|(name, _)| name == &key) {
                return Err(format!("line {number}: `{key}` is defined twice"));
            }
            let value = self.value(indent, number, &rest)?;
            entries.push((key, value));
        }
        Ok(Node::Map(entries))
    }

    fn value(&mut self, indent: usize, number: usize, rest: &str) -> Result<Node, String> {
        if rest.is_empty() {
            self.at += 1;
            self.skip();
            match self.lines.get(self.at) {
                Some(next) if next.indent > indent => self.block(next.indent),
                _ => Ok(Node::Scalar(String::new())),
            }
        } else if rest.starts_with('|') {
            self.at += 1;
            Ok(Node::Scalar(self.block_scalar(indent)))
        } else {
            self.at += 1;
            scalar(rest, number).map(Node::Scalar)
        }
    }

    /// A `|` block scalar: every following line indented past its key, dedented as one block.
    fn block_scalar(&mut self, parent: usize) -> String {
        let start = self.at;
        let mut end = start;
        let mut indent = usize::MAX;
        while let Some(line) = self.lines.get(end) {
            if line.blank() {
                end += 1;
                continue;
            }
            if line.indent <= parent {
                break;
            }
            indent = indent.min(line.indent);
            end += 1;
        }
        if indent == usize::MAX {
            indent = parent + 1;
        }
        let mut text = String::new();
        for line in &self.lines[start..end] {
            if line.blank() {
                text.push('\n');
                continue;
            }
            text.push_str(line.raw.get(indent..).unwrap_or_default());
            text.push('\n');
        }
        self.at = end;
        text.trim_end_matches('\n').to_string()
    }

    fn sequence(&mut self, indent: usize) -> Result<Node, String> {
        let mut items = Vec::new();
        loop {
            self.skip();
            let Some(line) = self.lines.get(self.at) else { break };
            if line.indent < indent {
                break;
            }
            if line.indent > indent {
                return Err(format!(
                    "line {}: {} spaces of indentation where {indent} or less were expected",
                    line.number, line.indent
                ));
            }
            let number = line.number;
            let text = line.text().to_string();
            let Some(rest) = text.strip_prefix('-') else { break };
            if !rest.is_empty() && !rest.starts_with(' ') {
                return Err(format!("line {number}: a sequence entry is `- `, and this is `{text}`"));
            }
            let inline = rest.trim_start().to_string();
            if inline.is_empty() {
                self.at += 1;
                self.skip();
                match self.lines.get(self.at) {
                    Some(next) if next.indent > indent => items.push(self.block(next.indent)?),
                    _ => items.push(Node::Scalar(String::new())),
                }
            } else if split_key(&inline).is_some() {
                let inner = indent + 2;
                self.lines[self.at] = Line {
                    number,
                    indent: inner,
                    raw: format!("{}{inline}", " ".repeat(inner)),
                };
                items.push(self.mapping(inner)?);
            } else {
                self.at += 1;
                items.push(Node::Scalar(scalar(&inline, number)?));
            }
        }
        Ok(Node::List(items))
    }
}

/// `key: rest`, when the line is a mapping entry at all.
fn split_key(text: &str) -> Option<(String, String)> {
    let colon = text.find(':')?;
    let key = &text[..colon];
    if key.is_empty()
        || !key
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || matches!(character, '_' | '-' | '.'))
    {
        return None;
    }
    let rest = &text[colon + 1..];
    if rest.is_empty() {
        return Some((key.to_string(), String::new()));
    }
    let rest = rest.strip_prefix(' ')?;
    Some((key.to_string(), rest.trim_end().to_string()))
}

/// A value on one line: plain, single-quoted or double-quoted. Escapes are not part of the subset.
fn scalar(text: &str, number: usize) -> Result<String, String> {
    if let Some(inner) = text.strip_prefix('"') {
        let Some(inner) = inner.strip_suffix('"') else {
            return Err(format!("line {number}: a double-quoted value is not closed"));
        };
        return Ok(inner.to_string());
    }
    if let Some(inner) = text.strip_prefix('\'') {
        let Some(inner) = inner.strip_suffix('\'') else {
            return Err(format!("line {number}: a single-quoted value is not closed"));
        };
        return Ok(inner.replace("''", "'"));
    }
    if text.starts_with('[') || text.starts_with('{') {
        return Err(format!(
            "line {number}: flow collections are not part of this reader; use block entries"
        ));
    }
    if text.starts_with('>') {
        return Err(format!("line {number}: folded scalars are not part of this reader; use `|`"));
    }
    if text.starts_with('&') || text.starts_with('*') || text.starts_with('!') {
        return Err(format!("line {number}: anchors, aliases and tags are not part of this reader"));
    }
    if text.contains(": ") {
        return Err(format!("line {number}: a plain value cannot contain `: `; quote it"));
    }
    Ok(text.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    const FORM: &str = "name: Bug report\ndescription: What went wrong.\nlabels:\n  - bug\nbody:\n  - type: input\n    id: os\n    attributes:\n      label: Operating system\n    validations:\n      required: true\n";

    #[test]
    fn a_form_reads_as_nested_entries() {
        let node = parse(FORM).expect("a form is readable");
        assert_eq!(node.field("labels").and_then(Node::as_list).map(<[Node]>::len), Some(1));
        let body = node.field("body").and_then(Node::as_list).expect("a body");
        assert_eq!(body[0].field("id").and_then(Node::as_str), Some("os"));
        let required = body[0].field("validations").and_then(|node| node.field("required"));
        assert_eq!(required.and_then(Node::as_str), Some("true"));
    }

    #[test]
    fn a_block_scalar_keeps_its_lines_and_drops_the_indentation() {
        let node = parse("value: |\n  one\n  two\n").expect("a block scalar is readable");
        assert_eq!(node.field("value").and_then(Node::as_str), Some("one\ntwo"));
    }

    #[test]
    fn a_tab_indent_is_refused_with_its_line() {
        let error = parse("a:\n\tb: 1\n").expect_err("a tab is not indentation");
        assert!(error.contains("line 2"), "{error}");
    }

    #[test]
    fn a_flow_collection_is_refused_with_its_line() {
        let error = parse("labels: [bug]\n").expect_err("flow lists are not the subset");
        assert!(error.contains("line 1"), "{error}");
    }

    #[test]
    fn a_plain_value_cannot_hold_a_colon_and_a_space() {
        let error = parse("description: one: two\n").expect_err("that is not a valid plain scalar");
        assert!(error.contains("quote it"), "{error}");
    }

    #[test]
    fn a_quoted_value_may_hold_one() {
        let node = parse("description: \"one: two\"\n").expect("quoting makes it a scalar");
        assert_eq!(node.field("description").and_then(Node::as_str), Some("one: two"));
    }

    #[test]
    fn a_key_defined_twice_is_refused() {
        let error = parse("a: 1\na: 2\n").expect_err("a duplicate key is not a document");
        assert!(error.contains("defined twice"), "{error}");
    }

    #[test]
    fn an_indented_document_is_refused() {
        let error = parse("  a: 1\n").expect_err("the document starts at column zero");
        assert!(error.contains("starts indented"), "{error}");
    }
}
