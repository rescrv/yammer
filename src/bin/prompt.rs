use arrrg::CommandLine;

use yammer::PromptOptions;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    minimal_signals::install();
    minimal_signals::block();
    let (options, prompts) = PromptOptions::from_command_line("USAGE: prompt [OPTIONS] [PROMPT]");
    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap()
        .block_on(yammer::prompt(options, &prompts))?;
    Ok(())
}
