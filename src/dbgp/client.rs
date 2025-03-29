use core::str;

use tokio::{
    io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader},
    net::TcpStream,
};
use xmltree::Element;

#[derive(Debug)]
pub struct Init {
    pub fileuri: String,
}

#[derive(Debug)]
pub struct Response {
    pub transaction_id: String,
    pub command: CommandResponse,
}

#[derive(Debug)]
pub enum CommandResponse {
    StepInto(ContinuationResponse),
    Run(ContinuationResponse),
    Unknown,
    StackGet(StackGetResponse),
}

#[derive(Debug)]
pub struct ContinuationResponse {
    pub status: String,
    pub reason: String,
}

#[derive(Debug)]
pub struct StackGetResponse {
    pub filename: String,
    pub line: u32,
}

#[derive(Debug)]
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

        return parse_xml(String::from_utf8(xml).unwrap().as_str());
    }

    pub(crate) async fn run(&mut self) -> Result<ContinuationResponse, anyhow::Error> {
        match self.command("run", &mut vec![]).await? {
            Message::Response(r) => match r.command {
                CommandResponse::Run(s) => Ok(s),
                _ => Err(anyhow::anyhow!("Unexpected response"))
            },
            _ => Err(anyhow::anyhow!("Unexpected response")),
        }
    }

    pub(crate) async fn step_into(&mut self) -> Result<ContinuationResponse, anyhow::Error> {
        match self.command("step_into", &mut vec![]).await? {
            Message::Response(r) => match r.command {
                CommandResponse::StepInto(s) => Ok(s),
                _ => Err(anyhow::anyhow!("Unexpected response"))
            },
            _ => Err(anyhow::anyhow!("Unexpected response")),
        }
    }

    async fn command(&mut self, cmd: &str, args: &mut Vec<&str>) -> Result<Message, anyhow::Error> {
        let cmd_str = format!("{} -i {} {}", cmd, self.tid, args.join(" "));
        let bytes = [cmd_str.trim_end(), "\0"].concat();
        self.stream.write(bytes.as_bytes()).await.unwrap();
        self.tid += 1;
        self.read().await
    }

    pub(crate) fn disonnect(&mut self) {
        self.stream.shutdown();
    }
}

fn parse_xml(xml: &str) -> Result<Message, anyhow::Error> {
    println!("Response : {}", xml);
    let root = Element::parse(xml.as_bytes())?;
    let attributes = root.attributes;
    match root.name.as_str() {
        "init" => Ok(Message::Init(Init {
            fileuri: attributes
                .get("fileuri")
                .expect("Expected fileuri to be set")
                .to_string(),
        })),
        "response" => Ok(Message::Response(Response {
            transaction_id: attributes
                .get("transaction_id")
                .expect("Expected transaction_id to be set")
                .to_string(),
            command: match attributes
                .get("command")
                .expect("Expected command to be set")
                .as_str()
            {
                "step_into" => CommandResponse::StepInto(parse_continuation_response(&attributes)),
                "run" => CommandResponse::Run(parse_continuation_response(&attributes)),
                _ => CommandResponse::Unknown,
            },
        })),
        _ => Err(anyhow::anyhow!("Unexpected element: {}", root.name)),
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
                    _ => panic!("Could not parse step into"),
                };
            }
            _ => panic!("Did not parse"),
        };
        Ok(())
    }
}
