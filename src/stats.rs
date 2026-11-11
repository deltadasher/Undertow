//! System stats sampled from /proc, for wallpapers that ask for the "stats" feed.

use serde_json::{json, Value};
use std::collections::HashMap;
use std::fs;

#[derive(Default)]
pub struct Sampler {
    prev_cpu: Vec<(u64, u64)>, // (total, idle); [0] is the aggregate line
    prev_proc: HashMap<u32, u64>,
    prev_net: Option<(u64, u64)>,
    temp_path: Option<String>,
    ident: Option<(String, String)>,
}

fn read(path: &str) -> String {
    fs::read_to_string(path).unwrap_or_default()
}

fn find_cpu_temp() -> Option<String> {
    const WANTED: [&str; 4] = ["coretemp", "k10temp", "zenpower", "cpu_thermal"];
    for entry in fs::read_dir("/sys/class/hwmon").ok()?.flatten() {
        let base = entry.path();
        let name = fs::read_to_string(base.join("name")).unwrap_or_default();
        let input = base.join("temp1_input");
        if WANTED.contains(&name.trim()) && input.exists() {
            return Some(input.to_string_lossy().into());
        }
    }
    None
}

impl Sampler {
    pub fn new() -> Self {
        Self { temp_path: find_cpu_temp(), ..Default::default() }
    }

    /// Forget previous readings, so the next sample only primes the deltas.
    pub fn reset(&mut self) {
        self.prev_cpu.clear();
        self.prev_proc.clear();
        self.prev_net = None;
    }

    /// Returns `None` for the first call after a reset, when there is nothing to diff against.
    pub fn sample(&mut self) -> Option<Value> {
        // --- CPU ---------------------------------------------------------
        let now: Vec<(u64, u64)> = read("/proc/stat")
            .lines()
            .take_while(|l| l.starts_with("cpu"))
            .filter_map(|l| {
                let f: Vec<u64> = l.split_whitespace().skip(1).take(8).filter_map(|x| x.parse().ok()).collect();
                (f.len() == 8).then(|| (f.iter().sum(), f[3] + f[4]))
            })
            .collect();
        let have_cpu = !now.is_empty() && now.len() == self.prev_cpu.len();
        let mut total_use = 0.0;
        let mut cores = Vec::new();
        let mut jiffies_per_core = 0u64;
        if have_cpu {
            for (i, (&(total, idle), &(ptotal, pidle))) in now.iter().zip(&self.prev_cpu).enumerate() {
                let dt = total.saturating_sub(ptotal);
                let di = idle.saturating_sub(pidle);
                let usage = if dt > 0 { 1.0 - di as f64 / dt as f64 } else { 0.0 };
                if i == 0 {
                    total_use = usage;
                    jiffies_per_core = dt / (now.len() as u64 - 1).max(1);
                } else {
                    cores.push(usage);
                }
            }
        }
        self.prev_cpu = now;

        // --- Memory (kB) -------------------------------------------------
        let mem: HashMap<String, f64> = read("/proc/meminfo")
            .lines()
            .filter_map(|l| {
                let (key, rest) = l.split_once(':')?;
                Some((key.to_string(), rest.split_whitespace().next()?.parse().ok()?))
            })
            .collect();
        let m = |k: &str| mem.get(k).copied().unwrap_or(0.0);

        // --- Network (bytes/s, everything except lo) ---------------------
        let (mut rx, mut tx) = (0u64, 0u64);
        for line in read("/proc/net/dev").lines().skip(2) {
            let Some((iface, rest)) = line.split_once(':') else { continue };
            if iface.trim() == "lo" {
                continue;
            }
            let f: Vec<u64> = rest.split_whitespace().filter_map(|x| x.parse().ok()).collect();
            if f.len() >= 9 {
                rx += f[0];
                tx += f[8];
            }
        }
        let (drx, dtx) = match self.prev_net {
            Some((prx, ptx)) => (rx.saturating_sub(prx), tx.saturating_sub(ptx)),
            None => (0, 0),
        };
        self.prev_net = Some((rx, tx));

        // --- Top processes -----------------------------------------------
        let page_kb = 4.0; // Linux page size is 4 kB on every platform this targets
        let mut procs: Vec<(u32, String, f64, f64)> = Vec::new();
        let mut ticks_now = HashMap::new();
        if let Ok(dir) = fs::read_dir("/proc") {
            for entry in dir.flatten() {
                let Some(pid) = entry.file_name().to_str().and_then(|s| s.parse::<u32>().ok()) else { continue };
                let stat = read(&format!("/proc/{pid}/stat"));
                // "pid (comm) state ..." where comm may itself contain spaces and parens.
                let (Some(open), Some(close)) = (stat.find('('), stat.rfind(')')) else { continue };
                let f: Vec<&str> = stat[close + 1..].split_whitespace().collect();
                if f.len() < 22 {
                    continue;
                }
                let ticks = f[11].parse::<u64>().unwrap_or(0) + f[12].parse::<u64>().unwrap_or(0);
                ticks_now.insert(pid, ticks);
                let cpu = match self.prev_proc.get(&pid) {
                    Some(&prev) if jiffies_per_core > 0 && ticks >= prev => {
                        100.0 * (ticks - prev) as f64 / jiffies_per_core as f64
                    }
                    _ => 0.0,
                };
                let rss = f[21].parse::<f64>().unwrap_or(0.0) * page_kb;
                procs.push((pid, stat[open + 1..close].to_string(), cpu, rss));
            }
        }
        self.prev_proc = ticks_now;
        let nprocs = procs.len();
        procs.sort_by(|a, b| b.2.total_cmp(&a.2).then(b.3.total_cmp(&a.3)));
        procs.truncate(10);

        // --- Everything else ---------------------------------------------
        let load: Vec<f64> = read("/proc/loadavg").split_whitespace().take(3).filter_map(|x| x.parse().ok()).collect();
        let uptime: f64 = read("/proc/uptime").split_whitespace().next().and_then(|x| x.parse().ok()).unwrap_or(0.0);
        let temp = self.temp_path.as_ref().and_then(|p| read(p).trim().parse::<f64>().ok()).map(|t| t / 1000.0);
        let (host, kernel) = self.ident.get_or_insert_with(|| {
            (
                read("/proc/sys/kernel/hostname").trim().to_string(),
                read("/proc/sys/kernel/osrelease").trim().to_string(),
            )
        });

        if !have_cpu {
            return None;
        }
        Some(json!({
            "cpu": { "total": total_use, "cores": cores },
            "mem": {
                "total": m("MemTotal"), "used": m("MemTotal") - m("MemAvailable"),
                "swapTotal": m("SwapTotal"), "swapUsed": m("SwapTotal") - m("SwapFree"),
            },
            "net": { "rx": drx, "tx": dtx },
            "load": load, "uptime": uptime, "temp": temp,
            "host": host, "kernel": kernel, "nprocs": nprocs,
            "procs": procs.iter().map(|(pid, name, cpu, mem)| json!({
                "pid": pid, "name": name, "cpu": cpu, "mem": mem,
            })).collect::<Vec<_>>(),
        }))
    }
}
