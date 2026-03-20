mod egg;
mod intarsia;
mod shared;

#[cfg(feature = "rerun-metrics")]
use log::info;

#[cfg(feature = "rerun-metrics")]
fn init_logging() {
    let _ = env_logger::Builder::from_default_env()
        .filter_level(log::LevelFilter::Info)
        .try_init();
}

#[cfg(feature = "rerun-metrics")]
enum OnlyEngine {
    Egg,
    Intarsia,
}

#[cfg(feature = "rerun-metrics")]
fn parse_only_arg() -> Result<Option<OnlyEngine>, String> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let mut i = 0;
    let mut only = None;

    while i < args.len() {
        match args[i].as_str() {
            "--only" => {
                let Some(value) = args.get(i + 1) else {
                    return Err("Missing value for --only. Expected 'egg' or 'intarsia'.".into());
                };
                only = Some(match value.as_str() {
                    "egg" => OnlyEngine::Egg,
                    "intarsia" => OnlyEngine::Intarsia,
                    other => {
                        return Err(format!(
                            "Unknown value for --only: '{}'. Expected 'egg' or 'intarsia'.",
                            other
                        ));
                    }
                });
                i += 2;
            }
            other => {
                return Err(format!(
                    "Unknown argument '{}'. Supported: --only <egg|intarsia>.",
                    other
                ));
            }
        }
    }

    Ok(only)
}

#[cfg(feature = "rerun-metrics")]
fn main() {
    init_logging();

    match parse_only_arg() {
        Ok(Some(OnlyEngine::Egg)) => egg::run(),
        Ok(Some(OnlyEngine::Intarsia)) => intarsia::run(),
        Ok(None) => {
            info!("Running both math benchmark engines (egg, intarsia)");
            egg::run();
            intarsia::run();
        }
        Err(msg) => {
            eprintln!("{}", msg);
            eprintln!(
                "Usage: cargo run --example math-bench --features rerun-metrics -- [--only <egg|intarsia>]"
            );
            std::process::exit(2);
        }
    }
}

#[cfg(not(feature = "rerun-metrics"))]
fn main() {
    println!("Enable rerun-metrics feature to run this experiment.");
}
