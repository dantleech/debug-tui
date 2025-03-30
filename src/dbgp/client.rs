use core::{slice, str};

use crossterm::style::Attribute;
use tokio::{
    io::{split, AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader},
    net::TcpStream,
};
use xmltree::{Element, XMLNode};

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

#[derive(Debug,Clone)]
pub enum Message {
    Init(Init),
    Response(Response),
}

pub struct DbgpClient {
    tid: u32,
    stream: TcpStream,
}
impl DbgpClient {
    pub(crate) fn new(s: TcpStream) -> Self {
        Self { stream: s, tid: 0 }
    }

    pub(crate) async fn read(&mut self) -> Result<Message, anyhow::Error> {

        let xml = self.read_raw().await?;
        if xml.is_empty() {
            return Err(anyhow::anyhow!("Empty XML response"));
        }
        return parse_xml(xml.as_str());
    }

    pub(crate) async fn read_raw(&mut self) -> Result<String, anyhow::Error> {
        let mut length: Vec<u8> = Vec::new();
        let mut xml: Vec<u8> = Vec::new();
        let mut reader = BufReader::new(&mut self.stream);

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
        return Ok(String::from_utf8(xml)?);
    }

    pub(crate) async fn run(&mut self) -> Result<ContinuationResponse, anyhow::Error> {
        match self.command("run", &mut vec![]).await? {
            Message::Response(r) => match r.command {
                CommandResponse::Run(s) => Ok(s),
                _ => Err(anyhow::anyhow!("Unexpected response")),
            },
            _ => Err(anyhow::anyhow!("Unexpected response")),
        }
    }

    pub(crate) async fn step_into(&mut self) -> Result<ContinuationResponse, anyhow::Error> {
        match self.command("step_into", &mut vec![]).await? {
            Message::Response(r) => match r.command {
                CommandResponse::StepInto(s) => Ok(s),
                _ => Err(anyhow::anyhow!("Unexpected response")),
            },
            _ => Err(anyhow::anyhow!("Unexpected response")),
        }
    }

    pub(crate) async fn step_over(&mut self) -> Result<ContinuationResponse, anyhow::Error> {
        match self.command("step_over", &mut vec![]).await? {
            Message::Response(r) => match r.command {
                CommandResponse::StepOver(s) => Ok(s),
                _ => Err(anyhow::anyhow!("Unexpected response")),
            },
            _ => Err(anyhow::anyhow!("Unexpected response")),
        }
    }

    pub(crate) async fn get_stack(&mut self) -> Result<Option<StackGetResponse>, anyhow::Error> {
        match self.command("stack_get", &mut vec!["-n 0"]).await? {
            Message::Response(r) => match r.command {
                CommandResponse::StackGet(s) => Ok(s),
                _ => Err(anyhow::anyhow!("Unexpected response")),
            },
            _ => Err(anyhow::anyhow!("Unexpected response")),
        }
    }

    pub(crate) async fn source(&mut self, filename: String) -> Result<String, anyhow::Error> {
        match self
            .command("source", &mut vec![format!("-f {}", filename).as_str()])
            .await?
        {
            Message::Response(r) => match r.command {
                CommandResponse::Source(s) => Ok(s),
                _ => Err(anyhow::anyhow!("Unexpected response")),
            },
            _ => Err(anyhow::anyhow!("Unexpected response")),
        }
    }

    async fn command(&mut self, cmd: &str, args: &mut Vec<&str>) -> Result<Message, anyhow::Error> {
        self.command_raw(cmd, args).await;
        self.read().await
    }

    async fn command_raw(&mut self, cmd: &str, args: &mut Vec<&str>) -> () {
        let cmd_str = format!("{} -i {} {}", cmd, self.tid, args.join(" "));
        let bytes = [cmd_str.trim_end(), "\0"].concat();
        self.tid += 1;
        self.stream.write(bytes.as_bytes()).await.unwrap();
    }

    pub(crate) async fn disonnect(&mut self) {
        self.stream.shutdown().await.unwrap();
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

        self.command_raw(name.unwrap(), &mut args).await;
        self.read_raw().await
    }
}

fn parse_xml(xml: &str) -> Result<Message, anyhow::Error> {
    let root = Element::parse(xml.as_bytes())?;
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
                _ => CommandResponse::Unknown,
            },
        })),
        _ => Err(anyhow::anyhow!("Unexpected element: {}", root.name)),
    }
}
fn parse_source(element: &Element) -> Result<String, anyhow::Error> {
    match element.children.get(0) {
        Some(e) => match e {
            XMLNode::CData(d) => Ok(String::from_utf8(base64::decode(d).unwrap()).unwrap()),
            _ => Err(anyhow::anyhow!("Expected CDATA")),
        },
        None => Err(anyhow::anyhow!("Expected CDATA")),
    }
}

fn parse_stack_get(element: &Element) -> Option<StackGetResponse> {
    match element.get_child("stack") {
        None => None,
        Some(s) => Some(StackGetResponse {
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
        }),
    }
}

fn parse_continuation_response(
    attributes: &std::collections::HashMap<String, String>,
) -> ContinuationResponse {
    return ContinuationResponse {
        status: attributes
            .get("status")
            .expect("Expected status to be set")
            .to_string(),
        reason: attributes
            .get("reason")
            .expect("Expected reason to be set")
            .to_string(),
    };
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
                        assert_eq!("file:///app/test.php", s.filename)
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
}
