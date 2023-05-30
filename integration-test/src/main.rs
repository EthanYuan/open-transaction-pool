mod const_definition;
mod help;
mod tests_omni_lock;
mod tests_otx_lock;
mod utils;

use help::{setup, teardown};

use clap::Parser;

use std::panic;
use std::time::Instant;

#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct Args {
    /// Name of the test case
    #[clap(short, long)]
    test: Option<String>,
}

fn main() {
    if std::env::var("RUST_LOG").is_err() {
        // should recognize RUST_LOG_STYLE environment variable
        env_logger::Builder::from_default_env()
            .filter(None, log::LevelFilter::Info)
            .init();
    } else {
        env_logger::init();
    }

    let args = Args::parse();

    // Setup test environment
    let child_handlers = setup();

    let (mut count_ok, mut count_failed) = (0, 0);
    let mut summary = vec![];
    let now = Instant::now();

    let mut exec_test = |t: &IntegrationTest| {
        let result = panic::catch_unwind(|| {
            (t.test_fn)();
        });
        let flag = if result.is_ok() {
            count_ok += 1;
            "ok"
        } else {
            count_failed += 1;
            "FAILED"
        };
        println!("{} ... {}", t.name, flag);
        summary.push((t.name, flag))
    };

    match args.test.as_deref() {
        Some(name) => {
            let t = IntegrationTest::from_name(name);
            if let Some(t) = t {
                exec_test(t);
            }
        }
        _ => {
            for t in inventory::iter::<IntegrationTest> {
                exec_test(t);
            }
        }
    }

    let elapsed = now.elapsed();

    // Teardown test environment
    teardown(child_handlers);

    // Display result
    println!();
    println!("Summary:");
    summary.into_iter().for_each(|(name, flag)| {
        println!("{} ... {}", name, flag);
    });
    println!();
    println!("running {} tests", count_ok + count_failed);
    println!(
        "test result: {}. {} passed; {} failed; finished in {}s",
        if count_failed > 0 { "FAILED" } else { "ok" },
        count_ok,
        count_failed,
        elapsed.as_secs_f32()
    );
}

#[derive(Debug)]
pub struct IntegrationTest {
    pub name: &'static str,
    pub test_fn: fn(),
}

impl IntegrationTest {
    pub fn _all_test_names() -> Vec<&'static str> {
        inventory::iter::<IntegrationTest>
            .into_iter()
            .map(|x| x.name)
            .collect::<Vec<&str>>()
    }

    pub fn from_name<S: AsRef<str>>(test_name: S) -> Option<&'static IntegrationTest> {
        inventory::iter::<IntegrationTest>
            .into_iter()
            .find(|t| t.name == test_name.as_ref())
    }
}

inventory::collect!(IntegrationTest);
