mod paths;

use clap::{Args, Parser, Subcommand, ValueEnum};

#[derive(Parser, Debug)]
#[command(name = "blind")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    Run(RunArgs),
    Show(ShowArgs),
}

#[derive(ValueEnum, Clone, Debug)]
enum ModeArg {
    Rubric,
    Pairwise,
}

#[derive(Args, Debug)]
struct RunArgs {
    /// Bare flag reads ./models.yaml; --models <path> uses a different file.
    #[arg(long, num_args = 0..=1, default_missing_value = "models.yaml", default_value = "models.yaml")]
    models: String,

    #[arg(long)]
    prompts: String,

    #[arg(long)]
    judge: Option<String>,

    /// Bare flag resolves to ./rubric.yaml if it exists, else the built-in default (empty string here means "let the scheduler decide").
    #[arg(long, num_args = 0..=1, default_missing_value = "", default_value = "")]
    rubric: String,

    #[arg(long, value_enum, default_value_t = ModeArg::Rubric)]
    mode: ModeArg,

    /// Pairwise only: runs both blind and unblind conditions.
    #[arg(long)]
    compare: bool,
}

#[derive(Args, Debug)]
struct ShowArgs {
    run_id: String,

    #[arg(long)]
    annotate: Option<String>,
}

fn main() {
    let cli = Cli::parse();
    println!("{cli:#?}");
}
