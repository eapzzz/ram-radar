use std::{collections::HashMap, fs, path::Path, time::Instant};

#[derive(Clone, Default, Debug)]
pub struct CpuTicks {
    pub busy: u64,
    pub total: u64,
}
pub fn parse_cpu(line: &str) -> Option<CpuTicks> {
    let v: Vec<u64> = line
        .split_whitespace()
        .skip(1)
        .take(8)
        .map(str::parse)
        .collect::<Result<_, _>>()
        .ok()?;
    if v.len() < 4 {
        return None;
    }
    let total = v.iter().sum::<u64>();
    Some(CpuTicks {
        busy: total.saturating_sub(v[3] + v.get(4).copied().unwrap_or(0)),
        total,
    })
}
pub fn cpu_percent(old: &CpuTicks, new: &CpuTicks) -> f32 {
    let total = new.total.saturating_sub(old.total);
    if total == 0 {
        0.0
    } else {
        (new.busy.saturating_sub(old.busy) as f32 / total as f32 * 100.0).clamp(0.0, 100.0)
    }
}
pub fn counter_rate(old: u64, new: u64, seconds: f64) -> f64 {
    if seconds <= 0.0 || !seconds.is_finite() {
        0.0
    } else {
        new.saturating_sub(old) as f64 / seconds
    }
}
pub fn pressure(text: &str) -> Option<f32> {
    text.lines()
        .find(|l| l.starts_with("some "))?
        .split_whitespace()
        .find_map(|v| v.strip_prefix("avg10=")?.parse().ok())
}

#[derive(Clone, Default, Debug)]
pub struct Network {
    pub name: String,
    pub received: u64,
    pub sent: u64,
    pub down: f64,
    pub up: f64,
}
#[derive(Clone, Default, Debug)]
pub struct Disk {
    pub name: String,
    pub read: f64,
    pub write: f64,
    pub busy: f32,
}
#[derive(Clone, Default, Debug)]
pub struct Mount {
    pub path: String,
    pub filesystem: String,
    pub total: u64,
    pub available: u64,
}
#[derive(Clone, Default, Debug)]
pub struct Sensor {
    pub name: String,
    pub value: f64,
    pub unit: &'static str,
}
#[derive(Clone, Default, Debug)]
pub struct Gpu {
    pub name: String,
    pub driver: String,
    pub busy: Option<f32>,
    pub used: Option<u64>,
    pub total: Option<u64>,
}
#[derive(Clone, Default, Debug)]
pub struct Hardware {
    pub cpu: f32,
    pub cores: Vec<f32>,
    pub cpu_name: String,
    pub load: String,
    pub uptime: f64,
    pub networks: Vec<Network>,
    pub disks: Vec<Disk>,
    pub mounts: Vec<Mount>,
    pub sensors: Vec<Sensor>,
    pub gpus: Vec<Gpu>,
    pub battery: Option<(f32, String)>,
    pub pressure_cpu: Option<f32>,
    pub pressure_memory: Option<f32>,
    pub pressure_io: Option<f32>,
}
impl Hardware {
    pub fn down(&self) -> f64 {
        self.networks.iter().fold(0.0, |sum, n| sum + n.down)
    }
    pub fn up(&self) -> f64 {
        self.networks.iter().fold(0.0, |sum, n| sum + n.up)
    }
}
pub struct Collector {
    cpus: Vec<CpuTicks>,
    net: HashMap<String, (u64, u64)>,
    disks: HashMap<String, (u64, u64, u64)>,
    last: Instant,
    cpu_name: String,
    mounts: Vec<Mount>,
    round: u32,
}
fn read(path: impl AsRef<Path>) -> String {
    fs::read_to_string(path)
        .unwrap_or_default()
        .trim()
        .to_owned()
}
fn number(path: impl AsRef<Path>) -> Option<u64> {
    read(path).parse().ok()
}
fn entries(path: impl AsRef<Path>) -> Vec<std::path::PathBuf> {
    fs::read_dir(path)
        .map(|e| e.filter_map(Result::ok).map(|e| e.path()).collect())
        .unwrap_or_default()
}
impl Collector {
    pub fn new() -> Self {
        Self {
            cpus: vec![],
            net: HashMap::new(),
            disks: HashMap::new(),
            last: Instant::now(),
            cpu_name: read("/proc/cpuinfo")
                .lines()
                .find_map(|l| {
                    l.strip_prefix("model name")
                        .and_then(|s| s.split_once(':'))
                        .map(|(_, s)| s.trim().to_owned())
                })
                .unwrap_or_else(|| "Processor".into()),
            mounts: vec![],
            round: 0,
        }
    }
    pub fn sample(&mut self) -> Hardware {
        let now = Instant::now();
        let seconds = now.duration_since(self.last).as_secs_f64();
        self.last = now;
        let cpus: Vec<_> = read("/proc/stat")
            .lines()
            .filter(|l| l.starts_with("cpu"))
            .filter_map(parse_cpu)
            .collect();
        let usages: Vec<_> = cpus
            .iter()
            .enumerate()
            .map(|(i, c)| {
                self.cpus
                    .get(i)
                    .map(|old| cpu_percent(old, c))
                    .unwrap_or(0.0)
            })
            .collect();
        self.cpus = cpus;
        let mut h = Hardware {
            cpu: usages.first().copied().unwrap_or(0.0),
            cores: usages.into_iter().skip(1).collect(),
            cpu_name: self.cpu_name.clone(),
            load: read("/proc/loadavg")
                .split_whitespace()
                .take(3)
                .collect::<Vec<_>>()
                .join(" / "),
            uptime: read("/proc/uptime")
                .split_whitespace()
                .next()
                .and_then(|s| s.parse().ok())
                .unwrap_or(0.0),
            pressure_cpu: pressure(&read("/proc/pressure/cpu")),
            pressure_memory: pressure(&read("/proc/pressure/memory")),
            pressure_io: pressure(&read("/proc/pressure/io")),
            ..Hardware::default()
        };
        let mut next_net = HashMap::new();
        for line in read("/proc/net/dev").lines() {
            let Some((name, fields)) = line.split_once(':') else {
                continue;
            };
            let name = name.trim();
            if name == "lo" {
                continue;
            }
            let v: Vec<u64> = fields
                .split_whitespace()
                .filter_map(|v| v.parse().ok())
                .collect();
            if v.len() < 16 {
                continue;
            }
            let (down, up) = self
                .net
                .get(name)
                .map(|(r, t)| {
                    (
                        counter_rate(*r, v[0], seconds),
                        counter_rate(*t, v[8], seconds),
                    )
                })
                .unwrap_or_default();
            h.networks.push(Network {
                name: name.into(),
                received: v[0],
                sent: v[8],
                down,
                up,
            });
            next_net.insert(name.into(), (v[0], v[8]));
        }
        self.net = next_net;
        h.networks.sort_by(|a, b| a.name.cmp(&b.name));
        let mut next_disks = HashMap::new();
        for line in read("/proc/diskstats").lines() {
            let v: Vec<_> = line.split_whitespace().collect();
            if v.len() < 14 {
                continue;
            }
            let name = v[2];
            if !Path::new(&format!("/sys/block/{name}/device")).exists() {
                continue;
            }
            let n = |i: usize| v[i].parse::<u64>().unwrap_or(0);
            let current = (n(5) * 512, n(9) * 512, n(12));
            let (r, w, b) = self
                .disks
                .get(name)
                .map(|old| {
                    (
                        counter_rate(old.0, current.0, seconds),
                        counter_rate(old.1, current.1, seconds),
                        (counter_rate(old.2, current.2, seconds) / 10.0).min(100.0) as f32,
                    )
                })
                .unwrap_or_default();
            h.disks.push(Disk {
                name: name.into(),
                read: r,
                write: w,
                busy: b,
            });
            next_disks.insert(name.into(), current);
        }
        self.disks = next_disks;
        if self.round.is_multiple_of(15) {
            self.mounts = mounts();
        }
        self.round = self.round.wrapping_add(1);
        h.mounts = self.mounts.clone();
        for hw in entries("/sys/class/hwmon") {
            let chip = read(hw.join("name"));
            for path in entries(&hw) {
                let name = path.file_name().unwrap_or_default().to_string_lossy();
                let (unit, divisor) = if name.starts_with("temp") && name.ends_with("_input") {
                    ("°C", 1000.0)
                } else if name.starts_with("fan") && name.ends_with("_input") {
                    ("RPM", 1.0)
                } else if name.starts_with("power") && name.ends_with("_average") {
                    ("W", 1_000_000.0)
                } else {
                    continue;
                };
                let Ok(value) = read(&path).parse::<f64>() else {
                    continue;
                };
                let base = name.trim_end_matches("_input").trim_end_matches("_average");
                let label = read(hw.join(format!("{base}_label")));
                h.sensors.push(Sensor {
                    name: format!(
                        "{} / {}",
                        chip,
                        if label.is_empty() { base } else { &label }
                    ),
                    value: value / divisor,
                    unit,
                });
            }
        }
        h.sensors.sort_by(|a, b| a.name.cmp(&b.name));
        for card in entries("/sys/class/drm") {
            let name = card.file_name().unwrap_or_default().to_string_lossy();
            if !name.starts_with("card") || name.contains('-') {
                continue;
            }
            let dev = card.join("device");
            if !dev.exists() {
                continue;
            }
            let driver = fs::read_link(dev.join("driver"))
                .ok()
                .and_then(|p| p.file_name().map(|n| n.to_string_lossy().into_owned()))
                .unwrap_or_else(|| "Unknown driver".into());
            let product = read(dev.join("product_name"));
            let product = if product.is_empty() {
                pci_name(&read(dev.join("vendor")), &read(dev.join("device"))).unwrap_or_default()
            } else {
                product
            };
            h.gpus.push(Gpu {
                name: if product.is_empty() {
                    format!(
                        "{} {} ({})",
                        read(dev.join("vendor")),
                        read(dev.join("device")),
                        name
                    )
                } else {
                    product
                },
                driver,
                busy: number(dev.join("gpu_busy_percent")).map(|v| v as f32),
                used: number(dev.join("mem_info_vram_used")),
                total: number(dev.join("mem_info_vram_total")),
            });
        }
        for battery in entries("/sys/class/power_supply") {
            if read(battery.join("type")) == "Battery" {
                h.battery = number(battery.join("capacity"))
                    .map(|n| (n as f32, read(battery.join("status"))));
                break;
            }
        }
        h
    }
}
fn mounts() -> Vec<Mount> {
    let mut result = vec![];
    let mut seen = std::collections::HashSet::new();
    for line in read("/proc/self/mountinfo").lines() {
        let Some((left, right)) = line.split_once(" - ") else {
            continue;
        };
        let l: Vec<_> = left.split_whitespace().collect();
        let r: Vec<_> = right.split_whitespace().collect();
        if l.len() < 5
            || r.len() < 2
            || !matches!(
                r[0],
                "ext4" | "ext3" | "btrfs" | "xfs" | "vfat" | "f2fs" | "ntfs3" | "exfat"
            )
            || !seen.insert(l[2].to_owned())
        {
            continue;
        }
        let path = l[4]
            .replace("\\040", " ")
            .replace("\\011", "\t")
            .replace("\\134", "\\");
        let Ok(c) = std::ffi::CString::new(path.as_str()) else {
            continue;
        };
        let mut stat = std::mem::MaybeUninit::<libc::statvfs>::uninit();
        // SAFETY: c is NUL terminated; stat points to writable, correctly sized storage.
        if unsafe { libc::statvfs(c.as_ptr(), stat.as_mut_ptr()) } != 0 {
            continue;
        }
        let stat = unsafe { stat.assume_init() };
        result.push(Mount {
            path,
            filesystem: r[0].into(),
            total: stat.f_blocks * stat.f_frsize,
            available: stat.f_bavail * stat.f_frsize,
        });
    }
    result
}

fn pci_name(vendor: &str, device: &str) -> Option<String> {
    static IDS: std::sync::OnceLock<String> = std::sync::OnceLock::new();
    let ids = IDS.get_or_init(|| {
        ["/usr/share/hwdata/pci.ids", "/usr/share/misc/pci.ids"]
            .iter()
            .find_map(|p| fs::read_to_string(p).ok())
            .unwrap_or_default()
    });
    let vendor = vendor.trim_start_matches("0x");
    let device = device.trim_start_matches("0x");
    let mut active = false;
    for line in ids.lines() {
        if !line.starts_with(['\t', '#']) && !line.is_empty() {
            active = line.starts_with(&format!("{vendor}  "));
        }
        if active {
            if let Some(name) = line.strip_prefix(&format!("\t{device}  ")) {
                return Some(name.into());
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn cpu_excludes_guest_ticks() {
        let c = parse_cpu("cpu 100 20 30 400 50 10 5 2 80 10").unwrap();
        assert_eq!(c.total, 617);
        assert_eq!(c.busy, 167);
    }
    #[test]
    fn cpu_delta_and_reset() {
        assert_eq!(
            cpu_percent(
                &CpuTicks {
                    busy: 10,
                    total: 20
                },
                &CpuTicks {
                    busy: 20,
                    total: 60
                }
            ),
            25.0
        );
        assert_eq!(
            cpu_percent(
                &CpuTicks {
                    busy: 30,
                    total: 60
                },
                &CpuTicks {
                    busy: 20,
                    total: 40
                }
            ),
            0.0
        );
    }
    #[test]
    fn rates_reset_and_zero_interval() {
        assert_eq!(counter_rate(100, 300, 2.0), 100.0);
        assert_eq!(counter_rate(300, 100, 2.0), 0.0);
        assert_eq!(counter_rate(0, 20, 0.0), 0.0);
    }
    #[test]
    fn missing_pressure_is_not_zero() {
        assert_eq!(pressure("some avg10=2.34 avg60=1.0 total=100"), Some(2.34));
        assert_eq!(pressure(""), None);
    }
    #[test]
    fn malformed_cpu_is_rejected() {
        assert!(parse_cpu("cpu 1 x").is_none());
    }
}
