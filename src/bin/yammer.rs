use arrrg::CommandLine;

use yammer::{
    Conversation, ConversationOptions, EmbedRequest, FieldWriteAccumulator, GenerateRequest,
    JsonAccumulator, PullRequest, Request, RequestOptions, ShowRequest,
};

fn usage() {
    eprintln!("USAGE: yammer [options] <command> [args]");
    std::process::exit(1);
}

#[tokio::main]
async fn main() -> Result<(), yammer::Error> {
    let (options, args) =
        RequestOptions::from_command_line_relaxed("USAGE: yammer [options] <command> [args]");
    if args.is_empty() {
        usage();
    }
    let args = args.iter().map(|s| s.as_str()).collect::<Vec<_>>();
    match args[0] {
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
            let (co, free) = ConversationOptions::from_arguments_relaxed(
                "USAGE: yammer [options] chat [chat-options]",
                &args[1..],
            );
            if !free.is_empty() {
                eprintln!("USAGE: yammer [options] chat [chat-options]");
                std::process::exit(1);
            }
            let conversation = Conversation::new(co);
            conversation.shell(options).await?;
        }
        _ => usage(),
    }
    Ok(())
}
