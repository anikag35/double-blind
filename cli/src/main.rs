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
    match &leaderboard.result {
        Some(pb::leaderboard::Result::Rubric(rubric)) => print_rubric_leaderboard(run_id, rubric),
        Some(pb::leaderboard::Result::Pairwise(counts)) => print_pairwise_counts(run_id, counts),
        None => println!("Run {run_id} has no results to show."),
    }
}

fn print_rubric_leaderboard(run_id: &str, rubric: &pb::RubricLeaderboard) {
    println!("\nRUBRIC LEADERBOARD — {run_id}");
    println!("────────────────────────────────");
    println!(" Rank  Model     Mean Score");
    for entry in &rubric.entries {
        println!("  {:<4} {:<9} {:.2}", entry.rank, entry.model, entry.mean_score);
    }
    println!("────────────────────────────────");
}

fn print_pairwise_counts(run_id: &str, counts: &pb::PairwiseCounts) {
    println!("\nPAIRWISE RESULTS — {run_id}");
    println!("Pair: {} vs {}", counts.model_a, counts.model_b);
    println!();

    // Column widths follow the model names' actual lengths, not a fixed
    // guess - a short "claude"/"gemini" pair and a long "claude-haiku-4-5"
    // pair both need to render with the numbers aligned under their header.
    let a_header = format!("{} wins", counts.model_a);
    let b_header = format!("{} wins", counts.model_b);
    let col_a = a_header.len();
    let col_b = b_header.len();

    println!("{:12}{a_header:<col_a$} {b_header:<col_b$} ties", "");

    let Some(blind) = &counts.blind else {
        println!("  (no results yet)");
        return;
    };
    let (aw, bw, t) = (blind.model_a_wins, blind.model_b_wins, blind.ties);
    println!("{:12}{aw:<col_a$} {bw:<col_b$} {t}", "blinded");

    if !counts.has_unblind {
        return;
    }
    let Some(unblind) = &counts.unblind else { return };
    let (aw, bw, t) = (unblind.model_a_wins, unblind.model_b_wins, unblind.ties);
    println!("{:12}{aw:<col_a$} {bw:<col_b$} {t}", "unblinded");

    let blind_total = blind.model_a_wins + blind.model_b_wins + blind.ties;
    let unblind_total = unblind.model_a_wins + unblind.model_b_wins + unblind.ties;
    if blind_total == 0 || unblind_total == 0 {
        return;
    }
    let blind_rate = 100.0 * blind.model_a_wins as f64 / blind_total as f64;
    let unblind_rate = 100.0 * unblind.model_a_wins as f64 / unblind_total as f64;
    println!();
    println!(
        "Raw win rate for {}: {:.0}% blinded -> {:.0}% unblinded ({:+.0} pts when identity was revealed).",
        counts.model_a,
        blind_rate,
        unblind_rate,
        unblind_rate - blind_rate
    );
    println!();
    println!("NOTE: this is a descriptive observation, not a significance test.");
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