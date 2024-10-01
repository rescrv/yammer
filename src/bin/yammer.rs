use std::time::{Duration, SystemTime};

use arrrg::CommandLine;

use yammer::{
    Conversation, ConversationOptions, CreateRequest, EmbedRequest, FieldWriteAccumulator,
    GenerateRequest, JsonAccumulator, PullRequest, Request, RequestOptions, ShowRequest,
};

// Environment variables
const YAMMER_LOG: &str = "YAMMER_LOG";
const YAMMER_HISTFILE: &str = "YAMMER_HISTFILE";

fn usage() {
    eprintln!("USAGE: yammer [options] <command> [args]");
    std::process::exit(1);
}

#[tokio::main]
async fn main() -> Result<(), yammer::Error> {
    minimal_signals::block();
    let (options, args) =
        RequestOptions::from_command_line_relaxed("USAGE: yammer [options] <command> [args]");
    if args.is_empty() {
        usage();
    }
    let args = args.iter().map(|s| s.as_str()).collect::<Vec<_>>();
    match args[0] {
        "debug" => {
            println!("{options:?}\nargs: {args:?}\nOLLAMA_HOST={}", options.url());
        }
        "pull" => {
            let (_, free) = PullRequest::from_arguments_relaxed(
                "USAGE: yammer [options] pull [model ...]",
                &args[1..],
            );
            for arg in &free {
                Request::pull(options.clone(), PullRequest::new(arg))?
                    .accumulate(&mut JsonAccumulator::new(std::io::stdout()))
                    .await?;
            }
        }
        "create" => {
            let (c, free) = CreateRequest::from_arguments_relaxed(
                "USAGE: yammer [options] create [create-options]",
                &args[1..],
            );
            if !free.is_empty() {
                eprintln!("command takes no positional arguments");
                std::process::exit(1);
            }
            Request::create(options.clone(), c)?
                .accumulate(&mut JsonAccumulator::new(std::io::stdout()))
                .await?;
        }
        "generate" => {
            let (g, free) = GenerateRequest::from_arguments_relaxed(
                "USAGE: yammer [options] generate [generate-options]",
                &args[1..],
            );
            if !free.is_empty() {
                eprintln!("command takes no positional arguments");
                std::process::exit(1);
            }
            Request::generate(options, g)?
                .accumulate(&mut FieldWriteAccumulator::new(
                    std::io::stdout(),
                    "response",
                ))
                .await?;
            println!();
        }
        "embed" => {
            let (e, free) = EmbedRequest::from_arguments_relaxed(
                "USAGE: yammer [options] generate [embed-options]",
                &args[1..],
            );
            if free.len() != 1 {
                eprintln!("USAGE: yammer [options] embed [embed-options] <file>");
                std::process::exit(1);
            }
            let inputs = free
                .iter()
                .map(std::fs::read_to_string)
                .collect::<Result<Vec<_>, _>>()?;
            Request::embed(options, e, inputs)?
                .accumulate(&mut JsonAccumulator::pretty(std::io::stdout()))
                .await?;
            println!();
        }
        "models" => {
            if args.len() != 1 {
                eprintln!("USAGE: yammer [options] tags");
                std::process::exit(1);
            }
            Request::tags(options)?
                .accumulate(&mut JsonAccumulator::pretty(std::io::stdout()))
                .await?;
        }
        "show" => {
            if args.len() != 2 {
                eprintln!("USAGE: yammer [options] show <model>");
                std::process::exit(1);
            }
            Request::show(options, ShowRequest::new(args[1]))?
                .accumulate(&mut JsonAccumulator::pretty(std::io::stdout()))
                .await?;
        }
        "chat" => {
            let (mut co, free) = ConversationOptions::from_arguments_relaxed(
                "USAGE: yammer [options] chat [chat-options]",
                &args[1..],
            );
            if !free.is_empty() {
                eprintln!("USAGE: yammer [options] chat [chat-options]");
                std::process::exit(1);
            }
            let log = co.log.take();
            co.log = file_for(&co, YAMMER_LOG, log);
            let histfile = co.histfile.take();
            co.histfile = file_for(&co, YAMMER_HISTFILE, histfile);
            let conversation = Conversation::new();
            conversation.shell(options, co).await?;
        }
        "replay" => {
            let (co, free) = ConversationOptions::from_arguments_relaxed(
                "USAGE: yammer [options] chat [chat-options]",
                &args[1..],
            );
            for arg in &free {
                let mut co = co.clone();
                let log = co.log.take();
                co.log = file_for(&co, YAMMER_LOG, log);
                let histfile = co.histfile.take();
                co.histfile = file_for(&co, YAMMER_HISTFILE, histfile);
                let mut conversation = Conversation::new();
                let msgs = yammer::load(arg)?;
                println!("loaded {} messages from {} for replay", msgs.len(), arg);
                for msg in msgs {
                    if msg.role == "user" {
                        conversation.push(msg);
                    }
                }
                println!("replaying conversation from {}: {conversation:?}", arg);
                conversation.replay(options.clone(), co.clone()).await?;
            }
        }
        _ => usage(),
    }
    Ok(())
}

fn file_for(co: &ConversationOptions, env_var: &str, log: Option<String>) -> Option<String> {
    let mut expanded = String::new();
    let mut prev = ' ';
    for c in log.or_else(|| std::env::var(env_var).ok())?.chars() {
        if prev == '%' {
            if c == 's' {
                expanded += &format!(
                    "{}",
                    SystemTime::now()
                        .duration_since(SystemTime::UNIX_EPOCH)
                        .unwrap_or(Duration::ZERO)
                        .as_secs()
                );
            } else if c == 'm' {
                expanded += co.model.as_str();
            } else if c == '%' {
                expanded.push('%');
            } else {
                expanded.push('%');
                expanded.push(c);
            }
            prev = ' ';
        } else {
            prev = c;
            if c != '%' {
                expanded.push(c);
            }
        }
    }
    if !expanded.is_empty() {
        Some(expanded)
    } else {
        None
    }
}
