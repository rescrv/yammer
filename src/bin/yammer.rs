use std::io::Write;
use std::path::PathBuf;

use arrrg::CommandLine;
use rustyline::error::ReadlineError;
use rustyline::history::FileHistory;
use rustyline::{Config, Editor};

use yammer::{
    ChatAccumulator, ChatMessage, ChatRequest, ChatResponse, Error, FieldWriteAccumulator,
    JsonAccumulator, Request, RequestOptions, VecAccumulator,
};

fn usage() {
    eprintln!("USAGE: yammer [options] <command> [args]");
    std::process::exit(1);
}

async fn pull(options: RequestOptions, model: &str) -> Result<(), Error> {
    let req = Request::pull(options, model)?;
    let acc = JsonAccumulator::new(std::io::stdout());
    yammer::accumulate(req, acc).await?;
    Ok(())
}

async fn generate(options: RequestOptions, model: &str, prompt: &str) -> Result<(), Error> {
    let req = Request::generate(options, model, prompt)?;
    let acc = FieldWriteAccumulator::new(std::io::stdout(), "response");
    yammer::accumulate(req, acc).await?;
    println!();
    Ok(())
}

async fn embed(
    options: RequestOptions,
    model: &str,
    files: &[impl AsRef<str>],
) -> Result<(), Error> {
    let inputs = files
        .iter()
        .map(|f| std::fs::read_to_string(f.as_ref()))
        .collect::<Result<Vec<_>, _>>()?;
    let req = Request::embed(options, model, inputs)?;
    let acc = JsonAccumulator::pretty(std::io::stdout());
    yammer::accumulate(req, acc).await?;
    println!();
    Ok(())
}

async fn models(options: RequestOptions) -> Result<(), Error> {
    let req = Request::tags(options)?;
    let acc = JsonAccumulator::pretty(std::io::stdout());
    yammer::accumulate(req, acc).await?;
    Ok(())
}

async fn show(options: RequestOptions, model: &str) -> Result<(), Error> {
    let req = Request::show(options, model)?;
    let acc = JsonAccumulator::pretty(std::io::stdout());
    yammer::accumulate(req, acc).await?;
    Ok(())
}

async fn chat(options: RequestOptions, model: &str) -> Result<(), Error> {
    let config = Config::builder()
        .auto_add_history(true)
        .max_history_size(1_000_000)
        .expect("this should always work")
        .history_ignore_dups(false)
        .expect("this should always work")
        .history_ignore_space(false)
        .build();
    let history_path = PathBuf::from(".yammer.history");
    let history = rustyline::history::FileHistory::new();
    let mut rl: Editor<(), FileHistory> =
        Editor::with_history(config, history).expect("this should always work");
    if PathBuf::from(".yammer.history").exists() {
        rl.load_history(&history_path)
            .expect("this should always work");
    }
    let mut history = vec![];
    loop {
        let line = rl.readline("yammer> ");
        match line {
            Ok(line) => {
                history.push(ChatMessage {
                    role: "user".to_string(),
                    content: line,
                    images: None,
                    tool_calls: None,
                });
                let cr = ChatRequest {
                    model: model.to_string(),
                    messages: history.clone(),
                    stream: Some(true),
                    tools: None,
                    format: None,
                    keep_alive: None,
                };
                let req = match Request::chat(options.clone(), cr) {
                    Ok(req) => req,
                    Err(err) => {
                        eprintln!("could not chat: {}", err);
                        continue;
                    }
                };
                let mut msgs = vec![];
                let mut acc = VecAccumulator::new(&mut msgs);
                let mut printer = ChatAccumulator::default();
                if let Err(err) = yammer::accumulate(req, &mut (&mut acc, &mut printer)).await {
                    eprintln!("could not chat: {:?}", err);
                }
                println!();
            }
            Err(ReadlineError::Interrupted) => {}
            Err(ReadlineError::Eof) => {
                return Ok(());
            }
            Err(err) => {
                eprintln!("could not read line: {}", err);
            }
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), std::io::Error> {
    let (options, args) =
        RequestOptions::from_command_line("USAGE: yammer [options] <command> [args]");
    if args.is_empty() {
        usage();
    }
    match args[0].as_str() {
        "pull" => {
            if args.len() != 2 {
                println!("USAGE: yammer [options] pull <model>");
                std::process::exit(1);
            }
            if let Err(error) = pull(options, &args[1]).await {
                println!("could not pull: {error:?}");
                std::process::exit(2);
            }
        }
        "generate" => {
            if args.len() != 3 {
                println!("USAGE: yammer [options] generate <model> <prompt>");
                std::process::exit(1);
            }
            if let Err(error) = generate(options, &args[1], &args[2]).await {
                println!("could not generate: {error:?}");
                std::process::exit(2);
            }
        }
        "embed" => {
            if args.len() < 2 {
                eprintln!("USAGE: yammer [options] embed <model> [FILE ...]");
                std::process::exit(1);
            }
            if let Err(error) = embed(options, &args[1], &args[2..]).await {
                eprintln!("could not embed: {error:?}");
                std::process::exit(2);
            }
        }
        "chat" => {
            if args.len() != 2 {
                eprintln!("USAGE: yammer [options] generate <model>");
                std::process::exit(1);
            }
            if let Err(error) = chat(options, &args[1]).await {
                eprintln!("could not chat: {error:?}");
                std::process::exit(2);
            }
        }
        "models" => {
            if args.len() != 1 {
                eprintln!("USAGE: yammer [options] tags");
                std::process::exit(1);
            }
            if let Err(error) = models(options).await {
                eprintln!("could not list models: {error:?}");
                std::process::exit(2);
            }
        }
        "show" => {
            if args.len() != 2 {
                eprintln!("USAGE: yammer [options] show <model>");
                std::process::exit(1);
            }
            if let Err(error) = show(options, &args[1]).await {
                eprintln!("could not show model: {error:?}");
                std::process::exit(2);
            }
        }
        _ => usage(),
    }
    Ok(())
}
