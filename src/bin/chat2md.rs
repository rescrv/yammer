use arrrg::CommandLine;
use utf8path::Path;

use yammer::{chat_path, Chat, ChatOptions, WordWrap};

#[derive(Clone, Debug, Default, Eq, PartialEq, arrrg_derive::CommandLine)]
struct Chat2HtmlOptions {}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let (_options, free) =
        Chat2HtmlOptions::from_command_line_relaxed("USAGE: chat2html [OPTIONS] [CHAT_ID]");
    if free.len() != 1 {
        eprintln!("command takes exactly one positional argument");
        std::process::exit(1);
    }
    let path = chat_path(&free[0]).expect("could not convert chat id to path");
    let chat = Chat::new(Some(path), ChatOptions::default())?;
    let req = chat.into_request();
    for msg in req.messages {
        let mut ww = WordWrap::new(72);
        let mut wrapped = vec![];
        if msg.content.trim().is_empty() {
            continue;
        }
        ww.push(msg.content.clone(), &mut wrapped)?;
        println!("# {}\n{}\n", msg.role, String::from_utf8_lossy(&wrapped));
    }
    Ok(())
}
