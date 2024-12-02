use arrrg::CommandLine;
use utf8path::Path;

use yammer::ChatOptions;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    minimal_signals::install();
    minimal_signals::block();
    let (options, mut free) = ChatOptions::from_command_line("USAGE: chat [OPTIONS]");
    if free.len() > 1 {
        eprintln!("command takes at most one positional argument");
        std::process::exit(1);
    }
    match tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap()
        .block_on(yammer::chat_shell(free.pop().map(Path::from), options))
    {
        Ok(()) => {}
        Err(err) => {
            eprintln!("could not chat: {}", err);
            std::process::exit(2);
        }
    }
    Ok(())
}
