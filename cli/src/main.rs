mod models;
mod paths;
mod pb {
    tonic::include_proto!("blind");
}

use std::fs;

use clap::{Args, Parser, Subcommand, ValueEnum};
use pb::scheduler_client::SchedulerClient;
use pb::{Mode, RunId, RunRequest};

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

fn scheduler_address() -> String {
    std::env::var("SCHEDULER_ADDRESS").unwrap_or_else(|_| "http://localhost:50051".to_string())
}

async fn run_command(args: RunArgs) -> Result<(), Box<dyn std::error::Error>> {
    let models_path = paths::resolve_path(&args.models)?;
    let models_content = fs::read_to_string(&models_path)?;
    let models = models::parse_models(&models_content)?;

    let prompts_path = paths::resolve_path(&args.prompts)?;
    let rubric_path = paths::resolve_rubric_arg(&args.rubric)?;

    let mode = match args.mode {
        ModeArg::Rubric => Mode::Rubric,
        ModeArg::Pairwise => Mode::Pairwise,
    };

    let mut client = SchedulerClient::connect(scheduler_address()).await?;
    let response = client
        .submit_run(RunRequest {
            models,
            prompts_path,
            judge: args.judge.unwrap_or_default(),
            rubric_path,
            mode: mode as i32,
            compare: args.compare,
        })
        .await?;
    let run_id = response.into_inner().run_id;
    println!("Run ID: {run_id}");

    // Poll GetRun until every task is done, then print the leaderboard.
    loop {
        let leaderboard = client
            .get_run(RunId { run_id: run_id.clone() })
            .await?
            .into_inner();

        if leaderboard.all_tasks_done {
            print_leaderboard(&run_id, &leaderboard);
            break;
        }

        tokio::time::sleep(std::time::Duration::from_secs(1)).await;
    }

    Ok(())
}

fn print_leaderboard(run_id: &str, leaderboard: &pb::Leaderboard) {
    println!("\nRUBRIC LEADERBOARD — {run_id}");
    println!("────────────────────────────────");
    println!(" Rank  Model     Mean Score");
    for entry in &leaderboard.entries {
        println!("  {:<4} {:<9} {:.2}", entry.rank, entry.model, entry.mean_score);
    }
    println!("────────────────────────────────");
}

async fn show_command(args: ShowArgs) -> Result<(), Box<dyn std::error::Error>> {
    let mut client = SchedulerClient::connect(scheduler_address()).await?;
    let leaderboard = client
        .get_run(RunId { run_id: args.run_id.clone() })
        .await?
        .into_inner();

    if !leaderboard.all_tasks_done {
        println!("Run {} is still in progress.", args.run_id);
        return Ok(());
    }

    print_leaderboard(&args.run_id, &leaderboard);

    if let Some(annotate) = args.annotate {
        // --annotate isn't implemented on the scheduler side yet (no
        // pairwise calibration RPC exists) - flag it rather than silently
        // ignoring the flag.
        println!("\n(--annotate {annotate} is not yet supported)");
    }

    Ok(())
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();
    let result = match cli.command {
        Commands::Run(args) => run_command(args).await,
        Commands::Show(args) => show_command(args).await,
    };

    if let Err(e) = result {
        eprintln!("Error: {e}");
        std::process::exit(1);
    }
}