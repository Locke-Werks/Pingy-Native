//! Shared state and one ping task per target.
//!
//! The Qt side never blocks on the network: QML polls `snapshot()` a few times a
//! second, and the graph reads `series()` on every frame. Both take the lock
//! briefly and copy out.

use crate::config::{self, Config, Entry, Settings};
use crate::model::{Sample, Target};
use crate::ping;
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, MutexGuard, OnceLock};
use std::time::{Duration, Instant};
use tokio::task::JoinHandle;

pub mod clock {
    use super::*;
    static EPOCH: OnceLock<Instant> = OnceLock::new();

    /// Milliseconds on a monotonic clock shared by the pingers and the graph.
    pub fn now_ms() -> f64 {
        EPOCH.get_or_init(Instant::now).elapsed().as_secs_f64() * 1000.0
    }
}

pub struct State {
    pub settings: Settings,
    pub targets: Vec<Target>,
    pub running: bool,
    /// Samples sent before this instant are dropped, so a reply still in
    /// flight when Clear is pressed cannot reappear on the emptied graph.
    cleared_at: f64,
    next_id: u64,
}

static STATE: OnceLock<Mutex<State>> = OnceLock::new();
static WORKERS: OnceLock<Mutex<HashMap<u64, JoinHandle<()>>>> = OnceLock::new();
/// Bumped on every change the graph has to re-tessellate for.
static REVISION: AtomicU64 = AtomicU64::new(1);

pub fn state() -> MutexGuard<'static, State> {
    STATE
        .get_or_init(|| {
            let cfg = config::load();
            let cap = cfg.settings.history_length as usize;
            let mut targets = Vec::new();
            for (i, e) in cfg.targets.into_iter().enumerate() {
                let name = if e.display_name.trim().is_empty() { e.host.clone() } else { e.display_name };
                let mut t = Target::new(i as u64 + 1, e.host, name, cap);
                t.active = e.active;
                targets.push(t);
            }
            let next_id = targets.len() as u64 + 1;
            Mutex::new(State { settings: cfg.settings, targets, running: true, cleared_at: f64::MIN, next_id })
        })
        .lock()
        .unwrap_or_else(|e| e.into_inner())
}

fn workers() -> MutexGuard<'static, HashMap<u64, JoinHandle<()>>> {
    WORKERS.get_or_init(Default::default).lock().unwrap_or_else(|e| e.into_inner())
}

pub fn revision() -> u64 {
    REVISION.load(Ordering::Acquire)
}

fn bump() {
    REVISION.fetch_add(1, Ordering::AcqRel);
}

fn persist(st: &State) {
    config::save(&Config {
        settings: st.settings.clone(),
        targets: st
            .targets
            .iter()
            .map(|t| Entry { host: t.host.clone(), display_name: t.name.clone(), active: t.active })
            .collect(),
    });
}

/// Start a worker for every target. Called once after the runtime is up.
pub fn start() {
    let ids: Vec<u64> = state().targets.iter().map(|t| t.id).collect();
    for id in ids {
        spawn(id);
    }
}

fn spawn(id: u64) {
    let h = tokio::spawn(worker(id));
    if let Some(old) = workers().insert(id, h) {
        old.abort();
    }
}

fn kill(id: u64) {
    if let Some(h) = workers().remove(&id) {
        h.abort();
    }
}

async fn worker(id: u64) {
    let Some(host) = state().targets.iter().find(|t| t.id == id).map(|t| t.host.clone()) else { return };

    let mut pinger = loop {
        let err = match ping::resolve(&host).await {
            Ok(ip) => match ping::Pinger::new(ip) {
                Ok(p) => break p,
                Err(e) => e.to_string(),
            },
            Err(e) => format!("cannot resolve: {e}"),
        };
        if let Some(t) = state().targets.iter_mut().find(|t| t.id == id) {
            t.error = Some(err);
        }
        tokio::time::sleep(Duration::from_secs(5)).await;
    };
    if let Some(t) = state().targets.iter_mut().find(|t| t.id == id) {
        t.error = None;
    }

    // Absolute schedule: each send is due one interval after the previous due
    // time, so samples land evenly spaced regardless of how long a reply took.
    let mut due = Instant::now();
    loop {
        let (go, settings) = {
            let st = state();
            let Some(t) = st.targets.iter().find(|t| t.id == id) else { return };
            (st.running && t.active, st.settings.clone())
        };
        if go {
            let t_ms = clock::now_ms();
            let r = pinger.ping(settings.ttl, Duration::from_millis(u64::from(settings.timeout_ms))).await;
            let mut st = state();
            let stale = t_ms < st.cleared_at;
            let Some(t) = st.targets.iter_mut().find(|t| t.id == id) else { return };
            if !stale {
                t.push(Sample { t_ms, latency_ms: r.latency_ms, ttl: r.ttl });
            }
            drop(st);
            bump();
        }
        due += Duration::from_millis(u64::from(settings.interval_ms));
        let now = Instant::now();
        if due < now {
            // Fell behind (sleep, suspend, a long timeout): resync rather than burst.
            due = now;
        }
        tokio::time::sleep_until(due.into()).await;
    }
}

pub fn add(host: &str, name: &str) -> bool {
    let host = host.trim();
    if host.is_empty() {
        return false;
    }
    let id = {
        let mut st = state();
        if st.targets.iter().any(|t| t.host.eq_ignore_ascii_case(host)) {
            return false;
        }
        let id = st.next_id;
        st.next_id += 1;
        let name = if name.trim().is_empty() { host } else { name.trim() };
        let cap = st.settings.history_length as usize;
        st.targets.push(Target::new(id, host.to_owned(), name.to_owned(), cap));
        persist(&st);
        id
    };
    spawn(id);
    bump();
    true
}

pub fn remove(id: u64) {
    kill(id);
    let mut st = state();
    st.targets.retain(|t| t.id != id);
    persist(&st);
    drop(st);
    bump();
}

pub fn set_active(id: u64, active: bool) {
    let mut st = state();
    if let Some(t) = st.targets.iter_mut().find(|t| t.id == id) {
        t.active = active;
    }
    persist(&st);
    drop(st);
    bump();
}

pub fn set_running(running: bool) {
    state().running = running;
    bump();
}

/// Works paused or running: it empties the history and stats either way.
pub fn clear_history() {
    let mut st = state();
    st.cleared_at = clock::now_ms();
    for t in st.targets.iter_mut() {
        t.clear();
    }
    drop(st);
    bump();
}

pub fn apply_settings(s: Settings) {
    let s = s.clamped();
    let mut st = state();
    for t in st.targets.iter_mut() {
        t.set_capacity(s.history_length as usize);
    }
    st.settings = s;
    persist(&st);
    drop(st);
    bump();
}

pub fn shutdown() {
    for (_, h) in workers().drain() {
        h.abort();
    }
}

/// What the graph draws for one target: its timestamps and latencies, with
/// timeouts as negative values.
pub struct Series {
    pub color: usize,
    pub active: bool,
    pub times: Vec<f64>,
    pub values: Vec<f64>,
}

pub fn series(extra: usize) -> (Vec<Series>, Settings) {
    let st = state();
    let n = st.settings.history_length as usize + extra;
    let out = st
        .targets
        .iter()
        .enumerate()
        .map(|(i, t)| {
            let (times, values) = t.tail(n).map(|s| (s.t_ms, s.latency_ms.map_or(-1.0, f64::from))).unzip();
            Series { color: i, active: t.active, times, values }
        })
        .collect();
    (out, st.settings.clone())
}

pub fn snapshot_json() -> String {
    let st = state();
    let targets: Vec<_> = st
        .targets
        .iter()
        .enumerate()
        .map(|(i, t)| {
            let s = t.stats();
            serde_json::json!({
                "id": t.id,
                "host": t.host,
                "name": t.name,
                "active": t.active,
                "color": i,
                "error": t.error,
                "current": s.current,
                "average": s.average,
                "min": s.min,
                "max": s.max,
                "loss": s.loss_percent,
                "sent": s.sent,
                "ttl": t.history().last().and_then(|x| x.ttl),
            })
        })
        .collect();
    serde_json::json!({
        "running": st.running,
        "settings": st.settings,
        "targets": targets,
    })
    .to_string()
}
