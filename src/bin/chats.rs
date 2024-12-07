use arrrg::CommandLine;

use yammer::ChatsOptions;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    minimal_signals::install();
    minimal_signals::block();
    let (options, free) = ChatsOptions::from_command_line_relaxed("USAGE: chats [OPTIONS]");
    if !free.is_empty() {
        eprintln!("command takes no positional arguments");
        std::process::exit(1);
    }
    match tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap()
        .block_on(yammer::chats_shell(options))
    {
        Ok(()) => {}
        Err(err) => {
            eprintln!("could not manage chats: {}", err);
            std::process::exit(2);
        }
    }
    Ok(())
}
