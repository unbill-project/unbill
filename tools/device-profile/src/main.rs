//! Direct-device workloads. No console, AsymChannel, RPC, CLI processes or UI.
use anyhow::{Context as _, Result, bail, ensure};
use std::{
    path::PathBuf,
    sync::Arc,
    time::{Duration, Instant},
};
use tracing::Instrument as _;
use tracing_subscriber::{filter::filter_fn, prelude::*};
use unbill_device::{AccountView, UnbillDevice};
use unbill_device_profile::{Capture, TARGET};
use unbill_model::{
    Currency, DeviceLabel, LedgerDoc, LedgerId, NewBill, NewDevice, NewUser, Share, Timestamp,
    UserId,
};
use unbill_storage::LedgerStore;

#[derive(Debug)]
struct Options {
    output: PathBuf,
    trace: bool,
    store: String,
    scenario: String,
    ledgers: usize,
    bills: usize,
    iterations: usize,
    concurrency: usize,
    worker_threads: usize,
    idle_secs: u64,
    timeout_secs: u64,
}
impl Options {
    fn parse() -> Result<Self> {
        let mut opt = Self {
            output: PathBuf::from("target/device-profile"),
            trace: true,
            store: "sqlite".into(),
            scenario: "all".into(),
            ledgers: 10,
            bills: 10,
            iterations: 20,
            concurrency: 4,
            worker_threads: std::thread::available_parallelism()
                .map(usize::from)
                .unwrap_or(1),
            idle_secs: 5,
            timeout_secs: 120,
        };
        let mut args = std::env::args().skip(1);
        while let Some(flag) = args.next() {
            if flag == "--help" || flag == "-h" {
                println!(
                    "unbill-device-profile [--output DIR] [--store sqlite|memory]\n  [--scenario idle|read|write|auth|sync|all] [--ledgers N] [--bills N]\n  [--iterations N] [--concurrency N] [--worker-threads N] [--idle-secs N] [--timeout-secs N] [--no-trace]\nEach run creates a new directory. sync uses real Iroh/mDNS/relay networking.\nThe headless Cargo feature disables BLE only. Ctrl-C preserves partial traces."
                );
                std::process::exit(0);
            }
            if flag == "--no-trace" {
                opt.trace = false;
                continue;
            }
            let value = args
                .next()
                .with_context(|| format!("missing value for {flag}"))?;
            match flag.as_str() {
                "--output" => opt.output = value.into(),
                "--store" => opt.store = value,
                "--scenario" => opt.scenario = value,
                "--ledgers" => opt.ledgers = value.parse()?,
                "--bills" => opt.bills = value.parse()?,
                "--iterations" => opt.iterations = value.parse()?,
                "--concurrency" => opt.concurrency = value.parse()?,
                "--worker-threads" => opt.worker_threads = value.parse()?,
                "--idle-secs" => opt.idle_secs = value.parse()?,
                "--timeout-secs" => opt.timeout_secs = value.parse()?,
                _ => bail!("unknown option: {flag}"),
            }
        }
        ensure!(
            ["sqlite", "memory"].contains(&opt.store.as_str()),
            "invalid store"
        );
        ensure!(
            ["idle", "read", "write", "auth", "sync", "all"].contains(&opt.scenario.as_str()),
            "invalid scenario"
        );
        ensure!(
            opt.ledgers > 0
                && opt.iterations > 0
                && opt.concurrency > 0
                && opt.worker_threads > 0
                && opt.timeout_secs > 0,
            "counts and timeout must be positive"
        );
        Ok(opt)
    }
    fn includes(&self, scenario: &str) -> bool {
        self.scenario == "all" || self.scenario == scenario
    }
}

fn main() -> Result<()> {
    let options = Options::parse()?;
    if options.output.exists() {
        bail!(
            "output already exists; choose a fresh directory: {}",
            options.output.display()
        );
    }
    std::fs::create_dir_all(&options.output)?;
    let capture = Capture::create(&options.output.join("spans.jsonl"))?;
    let trace_enabled = options.trace;
    tracing_subscriber::registry()
        .with(
            capture
                .clone()
                .with_filter(filter_fn(move |m| trace_enabled && m.target() == TARGET)),
        )
        .with(
            tracing_subscriber::fmt::layer()
                .with_writer(std::io::stderr)
                .with_filter(filter_fn(|m| {
                    *m.level() <= tracing::Level::WARN && m.target() != TARGET
                })),
        )
        .try_init()?;
    let metadata = serde_json::json!({
        "clock_ticks_per_sec": command_output("getconf", &["CLK_TCK"]).parse::<u64>().ok(),
        "trace_enabled": options.trace,
        "schema": 1, "pid": std::process::id(), "os": std::env::consts::OS,
        "arch": std::env::consts::ARCH, "store": options.store, "scenario": options.scenario,
        "ledgers": options.ledgers, "bills_per_ledger": options.bills,
        "iterations_per_worker": options.iterations, "concurrency": options.concurrency,
        "worker_threads": options.worker_threads,
        "idle_secs": options.idle_secs, "timeout_secs": options.timeout_secs,
        "headless": cfg!(feature = "headless"), "debug_assertions": cfg!(debug_assertions),
        "git_commit": command_output("git", &["rev-parse", "HEAD"]),
        "git_dirty": !command_output("git", &["status", "--porcelain"]).trim().is_empty(),
        "rustc": command_output("rustc", &["--version"]),
        "command": std::env::args().collect::<Vec<_>>(),
    });
    std::fs::write(
        options.output.join("metadata.json"),
        serde_json::to_vec_pretty(&metadata)?,
    )?;
    let start = Instant::now();
    let result = {
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(options.worker_threads)
            .enable_all()
            .build()?;
        runtime.block_on(async {
            let sampler = tokio::spawn(sample_runtime(options.output.join("runtime.jsonl"), capture.clone()));
            let result = tokio::select! {
                result = tokio::time::timeout(Duration::from_secs(options.timeout_secs), run(&options)) => result.unwrap_or_else(|_| Err(anyhow::anyhow!("workload timed out"))),
                signal = tokio::signal::ctrl_c() => { signal?; Err(anyhow::anyhow!("capture interrupted")) }
            };
            sampler.abort();
            match sampler.await {
                Ok(Err(error)) if result.is_ok() => Err(error.context("runtime sampler failed")),
                Err(error) if !error.is_cancelled() && result.is_ok() => Err(error.into()),
                _ => result,
            }
        })
        // Dropping this runtime closes/cancels outstanding background spans before
        // checking the writer. Blocking closures retain their real execution lifetime.
    };
    capture.finish()?;
    std::fs::write(
        options.output.join("result.json"),
        serde_json::to_vec_pretty(&serde_json::json!({
            "success": result.is_ok(), "elapsed_secs": start.elapsed().as_secs_f64(),
            "error": result.as_ref().err().map(|e| format!("{e:#}")),
        }))?,
    )?;
    println!("Capture: {}", options.output.display());
    result
}
fn command_output(program: &str, args: &[&str]) -> String {
    std::process::Command::new(program)
        .args(args)
        .output()
        .ok()
        .filter(|out| out.status.success())
        .map(|out| String::from_utf8_lossy(&out.stdout).trim().to_owned())
        .unwrap_or_else(|| "unavailable".into())
}
async fn open_store(opt: &Options, name: &str) -> Result<Arc<dyn LedgerStore>> {
    Ok(if opt.store == "memory" {
        Arc::new(unbill_store_memory::InMemoryStore::default())
    } else {
        Arc::new(unbill_store_sqlite::SqliteStore::open(opt.output.join(name)).await?)
    })
}

async fn run(opt: &Options) -> Result<()> {
    let store = open_store(opt, "device-a").await?;
    // Fixtures are created before device ownership. Never bypass the actor once
    // the device opens. No real account/session or existing database is used.
    let peer_store = if opt.includes("sync") {
        Some(open_store(opt, "device-b").await?)
    } else {
        None
    };
    store.create_secret_key().await?;
    let node = store.get_device_id().await?;
    let now = Timestamp::now()?;
    let mut ids = Vec::new();
    if let Some(peer_store) = &peer_store {
        peer_store.create_secret_key().await?;
    }
    async {
        use argon2::{Argon2, PasswordHasher, password_hash::SaltString};
        let salt = SaltString::encode_b64(b"unbill-profile-fixture")
            .map_err(|e| anyhow::anyhow!(e.to_string()))?;
        let hash = Argon2::default()
            .hash_password(b"synthetic-profile-password", &salt)
            .map_err(|e| anyhow::anyhow!(e.to_string()))?
            .to_string();
        store.create_account("profile", &hash).await?;
        for index in 0..opt.ledgers {
            let id = LedgerId::new();
            ids.push(id);
            let mut doc = LedgerDoc::new(
                id,
                format!("Fixture {index}"),
                Currency::from_code("USD").context("USD currency")?,
                now,
            )?;
            doc.add_device(
                NewDevice {
                    node_id: node.clone(),
                    label: DeviceLabel::new("Profiler A".into())?,
                },
                now,
            )?;
            if let Some(peer_store) = &peer_store {
                doc.add_device(
                    NewDevice {
                        node_id: peer_store.get_device_id().await?,
                        label: DeviceLabel::new("Profiler B".into())?,
                    },
                    now,
                )?;
                // Peer starts at a common base; A gets bills after this fork.
                let mut base = LedgerDoc::from_bytes(&doc.save())?;
                peer_store.save_ledger(&id.to_string(), &mut base).await?;
            }
            let user = UserId::new();
            doc.add_user(
                NewUser {
                    user_id: user,
                    display_name: "Fixture user".into(),
                },
                now,
            )?;
            for _ in 0..opt.bills {
                doc.add_bill(
                    NewBill {
                        amount_cents: 1000,
                        description: "Synthetic bill".into(),
                        payers: vec![Share {
                            user_id: user,
                            shares: 1,
                        }],
                        payees: vec![Share {
                            user_id: user,
                            shares: 1,
                        }],
                        prev: vec![],
                    },
                    node.clone(),
                    now,
                )?;
            }
            store.save_ledger(&id.to_string(), &mut doc).await?;
            store.grant_ledger("profile", id).await?;
        }
        Ok::<_, anyhow::Error>(())
    }
    .instrument(tracing::trace_span!(target: "unbill::profile", "workload.fixture"))
    .await?;
    let device = UnbillDevice::open(Arc::clone(&store)).await?;
    drop(store);
    device
        .register("registration-fixture", "synthetic-profile-password")
        .await?;
    let token = device
        .login("profile", "synthetic-profile-password")
        .await?;
    let view = device.account_view("profile").await?;
    ensure!(
        view.list_ledgers().await?.len() == ids.len(),
        "fixture grants missing"
    );
    if opt.includes("idle") {
        tokio::time::sleep(Duration::from_secs(opt.idle_secs))
            .instrument(tracing::trace_span!(target: "unbill::profile", "workload.idle"))
            .await;
    }
    if opt.includes("read") || opt.includes("write") {
        let mut tasks = tokio::task::JoinSet::new();
        for _ in 0..opt.concurrency {
            let device = Arc::clone(&device);
            let token = token.clone();
            let count = opt.iterations;
            let read = opt.includes("read");
            let write = opt.includes("write");
            tasks.spawn(async move {
                let view = device.session_view(&token).await?;
                for _ in 0..count {
                    if read {
                        async {
                            view.list_ledgers().await?;
                            device.collect_peers().await?;
                            Ok::<_, anyhow::Error>(())
                        }
                        .instrument(
                            tracing::trace_span!(target: "unbill::profile", "workload.read"),
                        )
                        .await?;
                    }
                    if write {
                        write_once(&view)
                            .instrument(
                                tracing::trace_span!(target: "unbill::profile", "workload.write"),
                            )
                            .await?;
                    }
                }
                Ok::<_, anyhow::Error>(())
            });
        }
        while let Some(result) = tasks.join_next().await {
            result??;
        }
    }
    if opt.includes("auth") {
        for _ in 0..opt.iterations {
            async {
                let token = device
                    .login("profile", "synthetic-profile-password")
                    .await?;
                device.session_account(&token).await?;
                device.logout(&token).await?;
                Ok::<_, anyhow::Error>(())
            }
            .instrument(tracing::trace_span!(target: "unbill::profile", "workload.auth"))
            .await?;
        }
    }
    if let Some(peer_store) = peer_store {
        let peer = UnbillDevice::open(peer_store).await?;
        let accepting = {
            let peer = Arc::clone(&peer);
            tokio::spawn(async move { peer.accept_loop().await })
        };
        let sync = async {
            device
                .trigger_peer_sync(peer.device_id())
                .instrument(
                    tracing::trace_span!(target: "unbill::profile", "workload.sync_initial"),
                )
                .await?;
            for _ in 0..opt.iterations {
                device
                    .trigger_peer_sync(peer.device_id())
                    .instrument(
                        tracing::trace_span!(target: "unbill::profile", "workload.sync_unchanged"),
                    )
                    .await?;
            }
            Ok::<_, anyhow::Error>(())
        }
        .await;
        peer.close().await;
        accepting.abort();
        let _ = accepting.await;
        sync?;
    }
    device.close().await;
    Ok(())
}
async fn write_once(view: &AccountView) -> Result<()> {
    let mut metas = view.list_ledgers().await?;
    let meta = metas.first_mut().context("no fixture ledger")?;
    meta.updated_at = Timestamp::now()?;
    view.save_ledger_meta(meta).await?;
    Ok(())
}

async fn sample_runtime(path: PathBuf, capture: Capture) -> Result<()> {
    use std::io::Write as _;
    let mut file = std::fs::File::create(path)?;
    let period = Duration::from_millis(100);
    let mut timer = tokio::time::interval(period);
    timer.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
    loop {
        let scheduled = timer.tick().await;
        // Linux counters are process-wide, including all Tokio/blocking threads.
        // Other platforms retain heartbeat data and report missing counters.
        let status = std::fs::read_to_string("/proc/self/status").unwrap_or_default();
        let read = |name: &str| {
            status.lines().find_map(|line| {
                line.strip_prefix(name)
                    .and_then(|s| s.split_whitespace().next()?.parse::<u64>().ok())
            })
        };
        let stat = std::fs::read_to_string("/proc/self/stat").unwrap_or_default();
        let fields: Vec<_> = stat
            .rsplit_once(')')
            .map(|(_, tail)| tail.split_whitespace().collect())
            .unwrap_or_default();
        let counter = |index| fields.get(index).and_then(|s: &&str| s.parse::<u64>().ok());
        serde_json::to_writer(
            &mut file,
            &serde_json::json!({
                "elapsed_ms": capture.elapsed().as_secs_f64() * 1000.0,
                "scheduler_lag_ms": scheduled.elapsed().as_secs_f64() * 1000.0,
                "rss_kib": read("VmRSS:"), "threads": read("Threads:"),
                "user_ticks": counter(11), "system_ticks": counter(12),
            }),
        )?;
        file.write_all(b"\n")?;
    }
}
