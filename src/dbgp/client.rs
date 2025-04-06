use anyhow::Result;
use base64::engine::general_purpose;
use base64::Engine;
use core::str;
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
    StackGet(Option<StackGetResponse>),
    Source(String),
    ContextGet(ContextGetResponse),
}

#[derive(Debug, Clone, PartialEq)]
pub struct ContextGetResponse {
    pub properties: Vec<Property>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Property {
    pub name: String,
    pub fullname: String,
    pub classname: Option<String>,
    pub page: Option<u32>,
    pub pagesize: Option<u32>,
    pub property_type: String,
    pub facet: Option<String>,
    pub size: Option<u32>,
    pub children: Vec<Property>,
    pub key: Option<String>,
    pub address: Option<String>,
    pub encoding: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ContinuationResponse {
    pub status: String,
    pub reason: String,
}

#[derive(Debug, Clone)]
pub struct StackGetResponse {
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
            anyhow::bail!("Empty XML response");
        }
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
        Ok(String::from_utf8(xml)?)
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

    pub(crate) async fn step_into(&mut self) -> Result<ContinuationResponse> {
        match self.command("step_into", &mut vec![]).await? {
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

    pub(crate) async fn get_stack(&mut self) -> Result<Option<StackGetResponse>> {
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
        let bytes = [cmd_str.trim_end(), "\0"].concat();
        self.tid += 1;
        self.stream
            .as_mut()
            .unwrap()
            .write(bytes.as_bytes())
            .await
            .map_err(anyhow::Error::from)
    }

    pub(crate) async fn disonnect(&mut self) {
        if let Some(s) = &mut self.stream {
            s.shutdown().await.unwrap()
        };
    }

    pub(crate) async fn exec_raw(&mut self, cmd: String) -> Result<String, anyhow::Error> {
        let mut cmd = cmd.split_whitespace();
        let name = cmd.next();
        if name.is_none() {
            return Ok("<command was empty>".to_string());
        }
        let mut args: Vec<&str> = Vec::new();

        for arg in cmd {
            args.push(arg);
        }

        self.command_raw(name.unwrap(), &mut args).await?;
        self.read_raw().await
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
        Some(e) => match e {
            XMLNode::CData(d) => {
                Ok(String::from_utf8(general_purpose::STANDARD.decode(d).unwrap()).unwrap())
            }
            _ => anyhow::bail!("Expected CDATA"),
        },
        None => anyhow::bail!("Expected CDATA"),
    }
}

fn parse_context_get(element: &mut Element) -> Result<ContextGetResponse, anyhow::Error> {
    let mut properties: Vec<Property> = vec![];
    while let Some(mut child) = element.take_child("property") {
        let p = Property {
            name: child.attributes
                .get("name")
                .expect("Expected name to be set")
                .to_string(),
            fullname: child.attributes
                .get("name")
                .expect("Expected fullname to be set")
                .to_string(),
            classname: child.attributes.get("classname").map(|s| s.to_string()),
            page: child.attributes.get("page").map(|s| s.parse::<u32>().unwrap()),
            pagesize: child.attributes
                .get("pagesize")
                .map(|s| s.parse::<u32>().unwrap()),
            property_type: child.attributes
                .get("type")
                .expect("Expected property_type to be set")
                .to_string(),
            facet: child.attributes.get("facet").map(|s| s.to_string()),
            size: child.attributes.get("size").map(|s| s.parse::<u32>().unwrap()),
            key: child.attributes.get("key").map(|name| name.to_string()),
            address: child.attributes.get("address").map(|name| name.to_string()),
            encoding: child.attributes.get("encoding").map(|s| s.to_string()),
            children: parse_context_get(&mut child).unwrap().properties,
        };
        properties.push(p);
    }
    Ok(ContextGetResponse { properties })
}

fn parse_stack_get(element: &Element) -> Option<StackGetResponse> {
    element.get_child("stack").map(|s| StackGetResponse {
        filename: s
            .attributes
            .get("filename")
            .expect("Expected status to be set")
            .to_string(),
        line: s
            .attributes
            .get("lineno")
            .expect("Expected lineno to be set")
            .parse()
            .unwrap(),
    })
}

fn parse_continuation_response(
    attributes: &std::collections::HashMap<String, String>,
) -> ContinuationResponse {
    ContinuationResponse {
        status: attributes
            .get("status")
            .expect("Expected status to be set")
            .to_string(),
        reason: attributes
            .get("reason")
            .expect("Expected reason to be set")
            .to_string(),
    }
}

#[cfg(test)]
mod test {
    use super::*;

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
                        assert_eq!("file:///app/test.php", s.unwrap().filename)
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
            r#"<response xmlns="urn:debugger_protocol_v1" xmlns:xdebug="https://xdebug.org/dbgp/xdebug" command="context_get" transaction_id="4" context="0"><property name="$bar" fullname="$bar" type="string" size="3" encoding="base64"><![CDATA[Zm9v]]></property><property name="$true" fullname="$true" type="bool"><![CDATA[1]]></property><property name="$this" fullname="$this" type="object" classname="Foo" children="1" numchildren="2" page="0" pagesize="32"><property name="true" fullname="$this-&gt;true" facet="public" type="bool"><![CDATA[1]]></property><property name="bar" fullname="$this-&gt;bar" facet="public" type="string" size="3" encoding="base64"><![CDATA[Zm9v]]></property></property></response>"#,
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
                                    property_type: "string".to_string(),
                                    facet: None,
                                    size: Some(3),
                                    children: vec![],
                                    key: None,
                                    address: None,
                                    encoding: Some("base64".to_string()),
                                },
                                Property {
                                    name: "$true".to_string(),
                                    fullname: "$true".to_string(),
                                    classname: None,
                                    page: None,
                                    pagesize: None,
                                    property_type: "bool".to_string(),
                                    facet: None,
                                    size: None,
                                    children: vec![],
                                    key: None,
                                    address: None,
                                    encoding: None,
                                },
                                Property {
                                    name: "$this".to_string(),
                                    fullname: "$this".to_string(),
                                    classname: Some("Foo".to_string()),
                                    page: Some(0),
                                    pagesize: Some(32),
                                    property_type: "object".to_string(),
                                    facet: None,
                                    size: None,
                                    children: vec![
                                        Property {
                                            name: "true".to_string(),
                                            fullname: "true".to_string(),
                                            classname: None,
                                            page: None,
                                            pagesize: None,
                                            property_type: "bool".to_string(),
                                            facet: Some("public".to_string()),
                                            size: None,
                                            children: vec![],
                                            key: None,
                                            address: None,
                                            encoding: None
                                        },
                                        Property {
                                            name: "bar".to_string(),
                                            fullname: "bar".to_string(),
                                            classname: None,
                                            page: None,
                                            pagesize: None,
                                            property_type: "string".to_string(),
                                            facet: Some("public".to_string()),
                                            size: Some(3),
                                            children: vec![],
                                            key: None,
                                            address: None,
                                            encoding: Some("base64".to_string())
                                        }
                                    ],
                                    key: None,
                                    address: None,
                                    encoding: None
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
