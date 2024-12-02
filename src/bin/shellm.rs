use arrrg::CommandLine;

use yammer::ShellmOptions;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    minimal_signals::install();
    minimal_signals::block();
    let (options, mut promptfiles) =
        ShellmOptions::from_command_line("USAGE: shellm [OPTIONS] [FILE]");
    if promptfiles.is_empty() {
        promptfiles.push("-".to_string());
    }
    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap()
        .block_on(yammer::shellm(options, &promptfiles))?;
    Ok(())
}
