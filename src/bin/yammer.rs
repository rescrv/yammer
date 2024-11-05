//! Yammer is a command line interface to the ollama API.

use std::fs::OpenOptions;
use std::io::Write;
use std::time::{Duration, SystemTime};

use arrrg::CommandLine;

use yammer::{
    Conversation, ConversationOptions, CreateRequest, FieldWriteAccumulator, GenerateRequest,
    JsonAccumulator, PullRequest, Request, RequestOptions, ShowRequest, SignalChecker,
};

/////////////////////////////////////// Environment Variables //////////////////////////////////////

const YAMMER_LOG: &str = "YAMMER_LOG";
const YAMMER_HISTFILE: &str = "YAMMER_HISTFILE";

/////////////////////////////////////////////// usage //////////////////////////////////////////////

fn usage() {
    eprintln!(
        r#"USAGE: yammer [options] <command>

Commands:
yammer [global-options] debug
yammer [global-options] pull --model <model>
yammer [global-options] create --name <model> --modelfile <contents>
yammer [global-options] models
yammer [global-options] show <model>
yammer [global-options] chat --model <model> --system <system> --log <log> --histfile <histfile>

Global Options:
--url <url>          The URL of the OLLAMA server

Environment Variables:
YAMMER_LOG           The log file name.  The following format specifiers are recognized:

                     %s - the current time in seconds since the epoch
                     %m - the model name
                     %% - a literal '%'

                     Overrides --log for chat command.

YAMMER_HISFILE       The history file name.  The following format specifiers are recognized:

                     %s - the current time in seconds since the epoch
                     %m - the model name
                     %% - a literal '%

                     Overrides --histfile for chat command.

OLLAMA_HOST          The URL of the OLLAMA server

NOTE:  The chat command is meant to be the only interactive mode of working, so it is the only
command that logs or saves history.  I envision `yammer generate` to be used programmatically
within makefiles or scripts.
"#
    );
    std::process::exit(1);
}

/////////////////////////////////////////////// main ///////////////////////////////////////////////

fn main() -> Result<(), yammer::Error> {
    minimal_signals::block();
    minimal_signals::install();
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap()
        .block_on(async_main())
}

async fn async_main() -> Result<(), yammer::Error> {
    let (options, args) =
        RequestOptions::from_command_line_relaxed("USAGE: yammer [options] <command> [args]");
    if args.is_empty() {
        usage();
    }
    let args = args.iter().map(|s| s.as_str()).collect::<Vec<_>>();
    let mut signal = SignalChecker;
    match args[0] {
        "debug" => {
            println!("{options:?}\nargs: {args:?}\nOLLAMA_HOST={}", options.url());
        }
        "pull" => {
            let (p, free) = PullRequest::from_arguments_relaxed(
                "USAGE: yammer [options] pull --model <model>",
                &args[1..],
            );
            if !free.is_empty() {
                eprintln!("command takes no positional arguments");
                std::process::exit(1);
            }
            Request::pull(options.clone(), PullRequest::new(p.model))?
                .accumulate(&mut (&mut signal, &mut JsonAccumulator::new(std::io::stdout())))
                .await?;
        }
        "create" => {
            let (c, free) = CreateRequest::from_arguments_relaxed(
                "USAGE: yammer [options] create --name <model> --modelfile <contents>",
                &args[1..],
            );
            if !free.is_empty() {
                eprintln!("command takes no positional arguments");
                std::process::exit(1);
            }
            Request::create(options.clone(), c)?
                .accumulate(&mut (&mut signal, &mut JsonAccumulator::new(std::io::stdout())))
                .await?;
        }
        "models" => {
            if args.len() != 1 {
                eprintln!("USAGE: yammer [options] models");
                std::process::exit(1);
            }
            Request::tags(options)?
                .accumulate(&mut (&mut signal, &mut JsonAccumulator::pretty(std::io::stdout())))
                .await?;
        }
        "show" => {
            if args.len() != 2 {
                eprintln!("USAGE: yammer [options] show <model>");
                std::process::exit(1);
            }
            Request::show(options, ShowRequest::new(args[1]))?
                .accumulate(&mut (&mut signal, &mut JsonAccumulator::pretty(std::io::stdout())))
                .await?;
        }
        "generate" => {
            let (g, free) = GenerateRequest::from_arguments_relaxed(
                "USAGE: yammer [options] generate --model <model> --prompt <prompt>",
                &args[1..],
            );
            if !free.is_empty() {
                eprintln!("command takes no positional arguments");
                std::process::exit(1);
            }
            Request::generate(options, g)?
                .accumulate(&mut (
                    &mut signal,
                    &mut FieldWriteAccumulator::new(std::io::stdout(), "response"),
                ))
                .await?;
            println!();
        }
        "write" => {
            let (mut g, models) = GenerateRequest::from_arguments_relaxed(
                "USAGE yammer [options] write [model] [model] [model]",
                &args[1..],
            );
            let editor =
                std::env::var("EDITOR").map_err(|err| yammer::Error::Message(err.to_string()))?;
            let mut file = OpenOptions::new()
                .create(true)
                .write(true)
                .truncate(true)
                .open(".yammer.tmp")?;
            file.write_all(g.prompt.as_bytes())?;
            file.flush()?;
            file.sync_all()?;
            drop(file);
            let status = std::process::Command::new(editor)
                .arg(".yammer.tmp")
                .status()?;
            if Some(0) != status.code() {
                std::process::exit(1);
            }
            if minimal_signals::pending().ismember(minimal_signals::SIGCHLD) {
                minimal_signals::wait(minimal_signals::SIGCHLD.into());
            }
            g.prompt = std::fs::read_to_string(".yammer.tmp")?.trim().to_string();
            std::fs::remove_file(".yammer.tmp")?;
            for model in models {
                let mut g = g.clone();
                g.model = model;
                Request::generate(options.clone(), g)?
                    .accumulate(&mut (
                        &mut signal,
                        &mut FieldWriteAccumulator::new(std::io::stdout(), "response"),
                    ))
                    .await?;
                println!();
            }
        }
        "chat" => {
            let (mut co, free) = ConversationOptions::from_arguments_relaxed(
                "USAGE: yammer [options] chat --model <model> --system <system>",
                &args[1..],
            );
            if !free.is_empty() {
                eprintln!("command takes no positional arguments");
                std::process::exit(1);
            }
            let log = co.log.take();
            co.log = file_for(&co, YAMMER_LOG, log);
            let histfile = co.histfile.take();
            co.histfile = file_for(&co, YAMMER_HISTFILE, histfile);
            let conversation = Conversation::new();
            conversation.shell(options, co).await?;
        }
        _ => usage(),
    }
    Ok(())
}

pub fn file_for(co: &ConversationOptions, env_var: &str, log: Option<String>) -> Option<String> {
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
