use arrrg::CommandLine;

use yammer::OneshotOptions;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    minimal_signals::install();
    minimal_signals::block();
    let (options, models) = OneshotOptions::from_command_line("USAGE: oneshot [OPTIONS] [MODEL]");
    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap()
        .block_on(yammer::oneshot(options, &models))?;
    Ok(())
}
