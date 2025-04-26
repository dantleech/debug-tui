use std::collections::HashMap;

use anyhow::Result;
use ratatui::text::{Line, ToLine};
use tree_sitter::{Node, Parser, Tree, TreeCursor};

#[derive(Clone, Debug)]
pub struct Value {
    pub value: String,
}

#[derive(Clone, Debug)]
pub struct Position {
    pub row: usize,
    pub char: usize,
}

#[derive(Clone, Debug)]
pub struct Range {
    pub start: Position,
    pub end: Position,
}

#[derive(Clone, Debug)]
pub struct VariableRef {
    pub range: Range,
    pub name: String,
    pub value: Option<Value>,
}

type Row = HashMap<usize, VariableRef>;

#[derive(Clone, Debug)]
pub struct Analysis {
    lines: HashMap<usize,Row>,
}

impl Analysis {
    fn register(&mut self, variable: VariableRef) {
        let line = self.lines.entry(variable.range.end.row).or_insert(HashMap::new());
        line.insert(variable.range.start.char, variable);
        ()
    }

    pub fn row(&self, number: usize) -> Row {
        let value = self.lines.get(&number);
        if value.is_none() {
            return HashMap::new();
        }
        return value.unwrap().clone();
    }

    fn new() -> Self {
        Self{
            lines: HashMap::new(),
        }
    }
}

pub struct Analyser {
    analysis: Analysis,
}

impl Analyser {
    pub fn analyze<'a>(&mut self, source: &str) -> Result<Analysis> {
        self.analysis = Analysis::new();
        let tree = self.parse(&source);
        self.walk(&tree.root_node(), source);

        return Ok(self.analysis.clone());
    }

    fn parse(&mut self, source: &str) -> Tree{
        let mut parser = Parser::new();
        let language = tree_sitter_php::LANGUAGE_PHP;
        parser.set_language(&language.into()).unwrap();
        let tree = parser.parse(source, None).unwrap();
        return tree;

    }

    fn walk(&mut self, node: &Node, source: &str) {
        println!("{:?}", node.kind());
        let count = node.child_count();
        if node.kind() == "variable_name" {
            self.analysis.register(VariableRef{
                name: node.utf8_text(source.as_bytes()).unwrap().to_string(),
                range: Range{
                    start: Position {row: node.start_position().row, char: node.start_position().column},
                    end: Position {row: node.end_position().row, char: node.end_position().column},
                },
                value: None, 
            });
        }

        for index in 0..count {
            let child = node.child(index).unwrap();
            self.walk(&child, source);
        }
    }

    pub fn new() -> Self {
        Self { analysis: Analysis { lines: HashMap::new() } }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use pretty_assertions::{assert_eq, assert_ne};

    #[test]
    fn test_analyse() -> Result<(), anyhow::Error> {
        let source = r#"<?php
            $var1 = 'hello'; $var2 = 'bar';

            echo $var3;
            if ($var1 && $var2) {
                die($var2);
            }

            function foo($bar, $baz) {
            }

            echo "Hello World";
        "#;
        let analysis = Analyser::new().analyze(source)?;
        let line = analysis.row(1);
        panic!("{:?}", line);
        Ok(())
    }
}

