use std::collections::HashMap;

use anyhow::Result;
use tree_sitter::{Node, Parser, Tree};

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Value {
    pub value: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Position {
    pub row: usize,
    pub char: usize,
}

impl Position {
    pub fn new(row: usize, column: usize) -> Self {
        Self { row, char: column }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Range {
    pub start: Position,
    pub end: Position,
}

impl Range {
    
    pub fn new(start: Position, end: Position) -> Self {
        Range { start, end }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct VariableRef {
    pub range: Range,
    pub name: String,
    pub value: Option<Value>,
}

// variable's start char is the key
type Row = HashMap<usize, VariableRef>;

#[derive(Clone, Debug)]
pub struct Analysis {
    rows: HashMap<usize,Row>,
}

impl Analysis {
    fn register(&mut self, variable: VariableRef) {
        let line = self.rows.entry(variable.range.end.row).or_default();
        line.insert(variable.range.start.char, variable);
    }

    pub fn row(&self, number: usize) -> Row {
        let value = self.rows.get(&number);
        if value.is_none() {
            return Row::new();
        }
        value.unwrap().clone()
    }

    fn new() -> Self {
        Self{
            rows: HashMap::new(),
        }
    }
}

pub struct Analyser {
    analysis: Analysis,
}

impl Default for Analyser {
    fn default() -> Self {
        Self::new()
    }
}

impl Analyser {
    pub fn analyze(&mut self, source: &str) -> Result<Analysis> {
        self.analysis = Analysis::new();
        let tree = self.parse(source);
        self.walk(&tree.root_node(), source);

        Ok(self.analysis.clone())
    }

    fn parse(&mut self, source: &str) -> Tree{
        let mut parser = Parser::new();
        let language = tree_sitter_php::LANGUAGE_PHP;
        parser.set_language(&language.into()).unwrap();
        
        parser.parse(source, None).unwrap()

    }

    fn walk(&mut self, node: &Node, source: &str) {
        let count = node.child_count();
        if node.kind() == "variable_name" {
            let var_ref = VariableRef{
                name: node.utf8_text(source.as_bytes()).unwrap().to_string(),
                range: Range{
                    start: Position {row: node.start_position().row, char: node.start_position().column},
                    end: Position {row: node.end_position().row, char: node.end_position().column},
                },
                value: None, 
            };
            self.analysis.register(var_ref);
        }

        for index in 0..count {
            let child = node.child(index).unwrap();
            self.walk(&child, source);
        }
    }

    pub fn new() -> Self {
        Self { analysis: Analysis { rows: HashMap::new() } }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use pretty_assertions::assert_eq;

    #[test]
    fn test_analyse_vars() -> Result<(), anyhow::Error> {
        let source = r#"<?php
$var1 = 'hello'; $var2 = 'bar';
        "#;
        let analysis = Analyser::new().analyze(source)?;
        let line = analysis.row(1);
        assert_eq!(2, line.values().len());

        assert_eq!(&VariableRef{
            range: Range::new(Position::new(1,0), Position::new(1,5)),
            name: "$var1".to_string(),
            value: None,
        }, line.get(&0).unwrap());

        assert_eq!(&VariableRef{
            range: Range::new(Position::new(1,17), Position::new(1,22)),
            name: "$var2".to_string(),
            value: None,
        }, line.get(&17).unwrap());
        Ok(())
    }

    #[test]
    fn test_analyse_list() -> Result<(), anyhow::Error> {
        let source = r#"<?php
list($var1, $var2) = some_call();
        "#;
        let analysis = Analyser::new().analyze(source)?;
        let line = analysis.row(1);
        assert_eq!(2, line.values().len());

        assert_eq!(&VariableRef{
            range: Range::new(Position::new(1,5), Position::new(1,10)),
            name: "$var1".to_string(),
            value: None,
        }, line.get(&5).unwrap());
        Ok(())
    }
}

