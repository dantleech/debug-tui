use anyhow::Result;
use base64::engine::general_purpose;
use base64::Engine;
use core::str;
use std::fmt::Display;
use log::debug;
use tokio::io::AsyncBufReadExt;
use tokio::io::AsyncWriteExt;
use tokio::io::BufReader;
use tokio::net::TcpStream;
use xmltree::Element;
use xmltree::XMLNode;

#[derive(Debug, Clone)]
pub struct Init {
    pub fileuri: String,
}

#[derive(Debug, Clone)]
pub struct Response {
    pub transaction_id: String,
    pub command: CommandResponse,
}

#[derive(Debug, Clone)]
pub enum CommandResponse {
    StepInto(ContinuationResponse),
    StepOver(ContinuationResponse),
    Run(ContinuationResponse),
    Unknown,
    StackGet(StackGetResponse),
    Source(String),
    ContextGet(ContextGetResponse),
}

#[derive(Debug, Clone, PartialEq)]
pub struct ContextGetResponse {
    pub properties: Vec<Property>,
}

#[derive(PartialEq, Clone, Debug)]
pub enum PropertyType {
    Bool,
    Int,
    Float,
    String,
    Null,
    Array,
    Hash,
    Object,
    Resource,
    Undefined,
}

impl Display for PropertyType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

impl PropertyType {
    pub fn as_str(&self) -> &str {
        match self {
            PropertyType::Bool => "bool",
            PropertyType::Int => "int",
            PropertyType::Float => "float",
            PropertyType::String => "string",
            PropertyType::Null => "null",
            PropertyType::Array => "array",
            PropertyType::Hash => "hash",
            PropertyType::Object => "object",
            PropertyType::Resource => "resource",
            PropertyType::Undefined => "undefined",
        }
    }

    fn from_str(expect: &str) -> PropertyType {
        match expect {
            "bool" => Self::Bool,
            "int" => Self::Int,
            "float" => Self::Float,
            "string" => Self::String,
            "null" => Self::Null,
            "array" => Self::Array,
            "hash" => Self::Hash,
            "object" => Self::Object,
            "resource" => Self::Resource,
            _ => Self::Undefined,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Property {
    pub name: String,
    pub fullname: String,
    pub classname: Option<String>,
    pub page: Option<u32>,
    pub pagesize: Option<u32>,
    pub property_type: PropertyType,
    pub facet: Option<String>,
    pub size: Option<u32>,
    pub children: Vec<Property>,
    pub key: Option<String>,
    pub address: Option<String>,
    pub encoding: Option<String>,
    pub value: Option<String>,
}
impl Property {
    pub(crate) fn type_name(&self) -> String {
        match self.property_type {
            PropertyType::Object => self.classname.clone().unwrap_or("object".to_string()),
            _ => self.property_type.to_string(),
        }
    }
    pub(crate) fn value_is(&self, value: &str) -> bool {
        match &self.value {
            Some(v) => value == *v,
            None => false,
        }
    }
}

#[derive(Clone, Debug)]
pub enum ContinuationStatus {
    Break,
    Stopping,
    Unknown(String),
}

#[derive(Debug, Clone)]
pub struct ContinuationResponse {
    pub status: ContinuationStatus,
    pub reason: String,
}

#[derive(Debug, Clone)]
pub struct StackGetResponse {
    pub entries: Vec<StackEntry>,
}

impl StackGetResponse {
    pub fn depth(&self) -> usize {
        self.entries.len()
    }
}

impl StackGetResponse {
    pub fn top(&self) -> &StackEntry {
        self.entries
            .first()
            .expect("Expected at least one stack entry")
    }

    pub(crate) fn top_or_none(&self) -> Option<&StackEntry> {
        self.entries.first()
    }
}

#[derive(Debug, Clone)]
pub struct StackEntry {
    pub filename: String,
    pub line: u32,
}

#[derive(Debug, Clone)]
pub enum Message {
    Init(Init),
    Response(Response),
}

pub struct DbgpClient {
    tid: u32,
    stream: Option<TcpStream>,
}
impl DbgpClient {
    pub(crate) fn new(s: Option<TcpStream>) -> Self {
        Self { stream: s, tid: 0 }
    }

    pub fn is_connected(&self) -> bool {
        self.stream.is_some()
    }

    pub(crate) async fn read_and_parse(&mut self) -> Result<Message> {
        let xml = self.read_raw().await?;
        if xml.is_empty() {
            return Err(anyhow::anyhow!("Empty XML response"));
        }
        debug!("[dbgp] << {}", xml);
        parse_xml(xml.as_str())
    }

    pub(crate) async fn read_raw(&mut self) -> Result<String> {
        let mut length: Vec<u8> = Vec::new();
        let mut xml: Vec<u8> = Vec::new();
        let mut reader = BufReader::new(self.stream.as_mut().unwrap());

        // read length and subsequently ignore it
        reader.read_until(b'\0', &mut length).await?;

        // read data
        reader.read_until(b'\0', &mut xml).await?;

        // remove dangling null-byte
        if let Some(e) = xml.last() {
            if *e == b'\0' {
                xml.pop();
            }
        }
        let string = String::from_utf8(xml)?;
        debug!("[dbgp] << {}", string);
        Ok(string)
    }

    pub(crate) async fn run(&mut self) -> Result<ContinuationResponse> {
        match self.command("run", &mut vec![]).await? {
            Message::Response(r) => match r.command {
                CommandResponse::Run(s) => Ok(s),
                _ => anyhow::bail!("Unexpected response"),
            },
            _ => anyhow::bail!("Unexpected response"),
        }
    }

    pub(crate) async fn feature_set(&mut self, feature: &str, value: &str) -> Result<()> {
        match self
            .command("feature_set", &mut vec!["-n", feature, "-v", value])
            .await?
        {
            Message::Response(r) => match r.command {
                CommandResponse::Unknown => Ok(()),
                _ => anyhow::bail!("Unexpected response"),
            },
            _ => anyhow::bail!("Unexpected response"),
        }
    }

    pub(crate) async fn context_get(&mut self) -> Result<ContextGetResponse> {
        match self.command("context_get", &mut vec![]).await? {
            Message::Response(r) => match r.command {
                CommandResponse::ContextGet(s) => Ok(s),
                _ => anyhow::bail!("Unexpected response"),
            },
            _ => anyhow::bail!("Unexpected response"),
        }
    }

    pub(crate) async fn step_into(&mut self) -> Result<ContinuationResponse> {
        match self.command("step_into", &mut vec![]).await? {
            Message::Response(r) => match r.command {
                CommandResponse::StepInto(s) => Ok(s),
                _ => anyhow::bail!("Unexpected response"),
            },
            _ => anyhow::bail!("Unexpected response"),
        }
    }

    pub(crate) async fn step_out(&mut self) -> Result<ContinuationResponse> {
        match self.command("step_out", &mut vec![]).await? {
            Message::Response(r) => match r.command {
                CommandResponse::StepInto(s) => Ok(s),
                _ => anyhow::bail!("Unexpected response"),
            },
            _ => anyhow::bail!("Unexpected response"),
        }
    }

    pub(crate) async fn step_over(&mut self) -> Result<ContinuationResponse> {
        match self.command("step_over", &mut vec![]).await? {
            Message::Response(r) => match r.command {
                CommandResponse::StepOver(s) => Ok(s),
                _ => anyhow::bail!("Unexpected response"),
            },
            _ => anyhow::bail!("Unexpected response"),
        }
    }

    pub(crate) async fn get_stack(&mut self) -> Result<StackGetResponse> {
        match self.command("stack_get", &mut vec!["-n 0"]).await? {
            Message::Response(r) => match r.command {
                CommandResponse::StackGet(s) => Ok(s),
                _ => anyhow::bail!("Unexpected response"),
            },
            _ => anyhow::bail!("Unexpected response"),
        }
    }

    pub(crate) async fn source(&mut self, filename: String) -> Result<String> {
        match self
            .command("source", &mut vec![format!("-f {}", filename).as_str()])
            .await?
        {
            Message::Response(r) => match r.command {
                CommandResponse::Source(s) => Ok(s),
                _ => anyhow::bail!("Unexpected response"),
            },
            _ => anyhow::bail!("Unexpected response"),
        }
    }

    async fn command(&mut self, cmd: &str, args: &mut Vec<&str>) -> Result<Message> {
        self.command_raw(cmd, args).await?;
        self.read_and_parse().await
    }

    async fn command_raw(&mut self, cmd: &str, args: &mut Vec<&str>) -> Result<usize> {
        let cmd_str = format!("{} -i {} {}", cmd, self.tid, args.join(" "));
        debug!("[dbgp] >> {}", cmd_str);
        let bytes = [cmd_str.trim_end(), "\0"].concat();
        self.tid += 1;
        self.stream
            .as_mut()
            .unwrap()
            .write(bytes.as_bytes())
            .await
            .map_err(anyhow::Error::from)
    }

    pub(crate) async fn disonnect(&mut self) -> Result<(), anyhow::Error> {
        if let Some(s) = &mut self.stream {
            let res = s.shutdown().await.or_else(|e| anyhow::bail!(e.to_string()));
            self.stream = None;
            return res;
        };
        Ok(())
    }

    pub(crate) async fn connect(&mut self, s: TcpStream) -> Result<Init> {
        self.stream = Some(s);
        match self.read_and_parse().await? {
            crate::dbgp::client::Message::Init(i) => Ok(i),
            _ => anyhow::bail!("Unexpected response"),
        }
    }
}

fn parse_xml(xml: &str) -> Result<Message, anyhow::Error> {
    let mut root = Element::parse(xml.as_bytes())?;
    match root.name.as_str() {
        "init" => Ok(Message::Init(Init {
            fileuri: root
                .attributes
                .get("fileuri")
                .expect("Expected fileuri to be set")
                .to_string(),
        })),
        "response" => Ok(Message::Response(Response {
            transaction_id: root
                .attributes
                .get("transaction_id")
                .expect("Expected transaction_id to be set")
                .to_string(),
            command: match root
                .attributes
                .get("command")
                .expect("Expected command to be set")
                .as_str()
            {
                "step_into" => {
                    CommandResponse::StepInto(parse_continuation_response(&root.attributes))
                }
                "step_out" => {
                    CommandResponse::StepInto(parse_continuation_response(&root.attributes))
                }
                "step_over" => {
                    CommandResponse::StepOver(parse_continuation_response(&root.attributes))
                }
                "run" => CommandResponse::Run(parse_continuation_response(&root.attributes)),
                "stack_get" => CommandResponse::StackGet(parse_stack_get(&root)),
                "source" => CommandResponse::Source(parse_source(&root)?),
                "context_get" => CommandResponse::ContextGet(parse_context_get(&mut root)?),
                _ => CommandResponse::Unknown,
            },
        })),
        _ => anyhow::bail!("Unexpected element: {}", root.name),
    }
}
fn parse_source(element: &Element) -> Result<String, anyhow::Error> {
    match element.children.first() {
        Some(XMLNode::CData(e)) => {
            Ok(String::from_utf8(general_purpose::STANDARD.decode(e).unwrap()).unwrap())
        }
        _ => anyhow::bail!("Expected CDATA"),
    }
}

fn parse_context_get(element: &mut Element) -> Result<ContextGetResponse, anyhow::Error> {
    let mut properties: Vec<Property> = vec![];
    while let Some(mut child) = element.take_child("property") {
        let encoding = child.attributes.get("encoding").map(|s| s.to_string());
        let p = Property {
            name: child
                .attributes
                .get("name")
                .expect("Expected name to be set")
                .to_string(),
            fullname: child
                .attributes
                .get("name")
                .expect("Expected fullname to be set")
                .to_string(),
            classname: child.attributes.get("classname").map(|s| s.to_string()),
            page: child
                .attributes
                .get("page")
                .map(|s| s.parse::<u32>().unwrap()),
            pagesize: child
                .attributes
                .get("pagesize")
                .map(|s| s.parse::<u32>().unwrap()),
            property_type: PropertyType::from_str(
                child
                    .attributes
                    .get("type")
                    .expect("Expected property_type to be set"),
            ),
            facet: child.attributes.get("facet").map(|s| s.to_string()),
            size: child
                .attributes
                .get("size")
                .map(|s| s.parse::<u32>().unwrap()),
            key: child.attributes.get("key").map(|name| name.to_string()),
            address: child.attributes.get("address").map(|name| name.to_string()),
            encoding: encoding.clone(),
            children: parse_context_get(&mut child).unwrap().properties,
            value: match child.children.first() {
                Some(XMLNode::CData(cdata)) => Some(match encoding {
                    Some(encoding) => match encoding.as_str() {
                        "base64" => {
                            String::from_utf8(general_purpose::STANDARD.decode(cdata).unwrap())
                                .unwrap()
                        }
                        _ => cdata.to_string(),
                    },
                    _ => cdata.to_string(),
                }),
                _ => None,
            },
        };
        properties.push(p);
    }
    Ok(ContextGetResponse { properties })
}

fn parse_stack_get(element: &Element) -> StackGetResponse {
    let mut entries: Vec<StackEntry> = Vec::new();
    for ce in &element.children {
        let stack_el = match ce {
            XMLNode::Element(element) => element,
            _ => continue,
        };
        if stack_el.name != "stack" {
            continue;
        }
        let entry = StackEntry {
            filename: stack_el
                .attributes
                .get("filename")
                .expect("Expected status to be set")
                .to_string(),
            line: stack_el
                .attributes
                .get("lineno")
                .expect("Expected lineno to be set")
                .parse()
                .unwrap(),
        };
        entries.push(entry);
    }

    StackGetResponse { entries }
}

fn parse_continuation_response(
    attributes: &std::collections::HashMap<String, String>,
) -> ContinuationResponse {
    let status = attributes.get("status").expect("Expected status to be set");
    ContinuationResponse {
        status: match status.as_str() {
            "break" => ContinuationStatus::Break,
            "stopping" => ContinuationStatus::Stopping,
            _ => ContinuationStatus::Unknown(status.to_string()),
        },
        reason: attributes
            .get("reason")
            .expect("Expected reason to be set")
            .to_string(),
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use pretty_assertions::assert_eq;

    #[test]
    fn test_parse_xml() -> Result<(), anyhow::Error> {
        let result = parse_xml(
            r#"<?xml version="1.0" encoding="iso-8859-1"?>
            <init xmlns="urn:debugger_protocol_v1" xmlns:xdebug="https://xdebug.org/dbgp/xdebug" fileuri="file:///application/vendor/bin/codecept" language="PHP" xdebug:language_version="7.1.33-53+ubuntu22.04.1+deb.sury.org+1" protocol_version="1.0" appid="37"><engine version="2.9.8"><![CDATA[Xdebug]]></engine><author><![CDATA[Derick Rethans]]></author><url><![CDATA[https://xdebug.org]]></url><copyright><![CDATA[Copyright (c) 2002-2020 by Derick Rethans]]></copyright></init>
        "#,
        )?;
        match result {
            Message::Init(init) => {
                assert_eq!("file:///application/vendor/bin/codecept", init.fileuri);
            }
            _ => panic!("Did not parse"),
        }
        Ok(())
    }

    #[test]
    fn test_parse_get_stack() -> Result<(), anyhow::Error> {
        let result = parse_xml(
            r#"<?xml version="1.0" encoding="iso-8859-1"?>
<response xmlns="urn:debugger_protocol_v1" xmlns:xdebug="https://xdebug.org/dbgp/xdebug" command="stack_get" transaction_id="10"><stack where="call_function" level="0" type="file" filename="file:///app/test.php" lineno="6"></stack></response>"#,
        )?;

        match result {
            Message::Response(r) => {
                match r.command {
                    CommandResponse::StackGet(s) => {
                        assert_eq!("file:///app/test.php", s.top().filename)
                    }
                    _ => panic!("Could not parse get_stack"),
                };
            }
            _ => panic!("Did not parse"),
        };
        Ok(())
    }

    #[test]
    fn test_parse_get_multiple_stack_entries() -> Result<(), anyhow::Error> {
        let result = parse_xml(
            r#"<?xml version="1.0" encoding="iso-8859-1"?>
<response xmlns="urn:debugger_protocol_v1" xmlns:xdebug="https://xdebug.org/dbgp/xdebug" command="stack_get" transaction_id="16"><stack where="another_function" level="0" type="file" filename="file:///home/daniel/www/dantleech/debug-tui/php/test.php" lineno="21"></stack><stack where="call_function" level="1" type="file" filename="file:///home/daniel/www/dantleech/debug-tui/php/test.php" lineno="11"></stack><stack where="{main}" level="2" type="file" filename="file:///home/daniel/www/dantleech/debug-tui/php/test.php" lineno="4"></stack></response>"#,
        )?;

        match result {
            Message::Response(r) => {
                match r.command {
                    CommandResponse::StackGet(s) => {
                        assert_eq!(3, s.entries.len())
                    }
                    _ => panic!("Could not parse get_stack"),
                };
            }
            _ => panic!("Did not parse"),
        };
        Ok(())
    }

    #[test]
    fn test_parse_source() -> Result<(), anyhow::Error> {
        let result = parse_xml(
            r#"<response xmlns="urn:debugger_protocol_v1" xmlns:xdebug="https://xdebug.org/dbgp/xdebug" command="source" transaction_id="11" encoding="base64"><![CDATA[PD9waHAKCmNhbGxfZnVuY3Rpb24oImhlbGxvIik7CgpmdW5jdGlvbiBjYWxsX2Z1bmN0aW9uKHN0cmluZyAkaGVsbG8pIHsKICAgIGVjaG8gJGhlbGxvOwp9Cg==]]></response>"#,
        )?;

        match result {
            Message::Response(r) => {
                match r.command {
                    CommandResponse::Source(source) => {
                        let expected = r#"<?php

call_function("hello");

function call_function(string $hello) {
    echo $hello;
}
"#;
                        assert_eq!(expected, source)
                    }
                    _ => panic!("Could not parse get_stack"),
                };
            }
            _ => panic!("Did not parse"),
        };
        Ok(())
    }

    #[test]
    fn test_parse_context_get() -> Result<(), anyhow::Error> {
        let result = parse_xml(
            r#"
            <response xmlns="urn:debugger_protocol_v1" xmlns:xdebug="https://xdebug.org/dbgp/xdebug" command="context_get" transaction_id="4" context="0">
            <property name="$bar" fullname="$bar" type="string" size="3" encoding="base64"><![CDATA[Zm9v]]></property>
            <property name="$float" fullname="$float" type="float"><![CDATA[123.4]]></property>
            <property name="$int" fullname="$int" type="int"><![CDATA[123]]></property>
            <property name="$true" fullname="$true" type="bool"><![CDATA[1]]></property>
            <property name="$this" fullname="$this" type="object" classname="Foo" children="1" numchildren="2" page="0" pagesize="32">
                <property name="true" fullname="$this-&gt;true" facet="public" type="bool"><![CDATA[1]]></property>
                <property name="bar" fullname="$this-&gt;bar" facet="public" type="string" size="3" encoding="base64"><![CDATA[Zm9v]]></property>
            </property>
            </response>"#,
        )?;

        match result {
            Message::Response(r) => {
                match r.command {
                    CommandResponse::ContextGet(response) => {
                        let expected = ContextGetResponse {
                            properties: vec![
                                Property {
                                    name: "$bar".to_string(),
                                    fullname: "$bar".to_string(),
                                    classname: None,
                                    page: None,
                                    pagesize: None,
                                    property_type: PropertyType::String,
                                    facet: None,
                                    size: Some(3),
                                    children: vec![],
                                    key: None,
                                    address: None,
                                    encoding: Some("base64".to_string()),
                                    value: Some("foo".to_string()),
                                },
                                Property {
                                    name: "$float".to_string(),
                                    fullname: "$float".to_string(),
                                    classname: None,
                                    page: None,
                                    pagesize: None,
                                    property_type: PropertyType::Float,
                                    facet: None,
                                    size: None,
                                    children: vec![],
                                    key: None,
                                    address: None,
                                    encoding: None,
                                    value: Some("123.4".to_string()),
                                },
                                Property {
                                    name: "$int".to_string(),
                                    fullname: "$int".to_string(),
                                    classname: None,
                                    page: None,
                                    pagesize: None,
                                    property_type: PropertyType::Int,
                                    facet: None,
                                    size: None,
                                    children: vec![],
                                    key: None,
                                    address: None,
                                    encoding: None,
                                    value: Some("123".to_string()),
                                },
                                Property {
                                    name: "$true".to_string(),
                                    fullname: "$true".to_string(),
                                    classname: None,
                                    page: None,
                                    pagesize: None,
                                    property_type: PropertyType::Bool,
                                    facet: None,
                                    size: None,
                                    children: vec![],
                                    key: None,
                                    address: None,
                                    encoding: None,
                                    value: Some("1".to_string()),
                                },
                                Property {
                                    name: "$this".to_string(),
                                    fullname: "$this".to_string(),
                                    classname: Some("Foo".to_string()),
                                    page: Some(0),
                                    pagesize: Some(32),
                                    property_type: PropertyType::Object,
                                    facet: None,
                                    size: None,
                                    children: vec![
                                        Property {
                                            name: "true".to_string(),
                                            fullname: "true".to_string(),
                                            classname: None,
                                            page: None,
                                            pagesize: None,
                                            property_type: PropertyType::Bool,
                                            facet: Some("public".to_string()),
                                            size: None,
                                            children: vec![],
                                            key: None,
                                            address: None,
                                            encoding: None,
                                            value: Some("1".to_string()),
                                        },
                                        Property {
                                            name: "bar".to_string(),
                                            fullname: "bar".to_string(),
                                            classname: None,
                                            page: None,
                                            pagesize: None,
                                            property_type: PropertyType::String,
                                            facet: Some("public".to_string()),
                                            size: Some(3),
                                            children: vec![],
                                            key: None,
                                            address: None,
                                            encoding: Some("base64".to_string()),
                                            value: Some("foo".to_string()),
                                        },
                                    ],
                                    key: None,
                                    address: None,
                                    encoding: None,
                                    value: None,
                                },
                            ],
                        };
                        assert_eq!(expected, response)
                    }
                    _ => panic!("Could not parse context_get"),
                };
            }
            _ => panic!("Did not parse"),
        };
        Ok(())
    }
}
