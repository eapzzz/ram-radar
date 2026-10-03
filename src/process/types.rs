#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Category {
    Apps,
    Background,
    System,
}

impl Category {
    pub fn short_title(&self) -> &'static str {
        match self {
            Self::Apps => "Application",
            Self::Background => "Process group",
            Self::System => "Kernel",
        }
    }
}

#[derive(Debug, Clone)]
pub struct ProcessInfo {
    pub starttime: u64,
    pub pid: u32,
    pub ppid: u32,
    pub name: String,
    pub cmdline: String,
    pub exe: String,
    pub pss_kb: u64,
    pub rss_kb: u64,
    pub uss_kb: u64,
    pub swap_kb: u64,
    pub threads: u32,
    pub role_hint: String,
    #[allow(dead_code)]
    pub utime_stime: u64,
    pub cpu_usage: f32,
    /// True when `/proc/<pid>/smaps_rollup` was unreadable and `pss_kb` /
    /// `uss_kb` are RSS-derived upper bounds rather than real PSS figures.
    /// Surfaced in the UI so the headline "real RAM" number is not silently
    /// inflated by processes we lack permission to measure.
    pub pss_estimated: bool,
}

#[derive(Debug, Clone)]
pub struct AppGroup {
    pub key: String,
    pub display_name: String,
    pub category: Category,
    pub processes: Vec<ProcessInfo>,
    pub total_pss_kb: u64,
    pub total_rss_kb: u64,
    pub total_uss_kb: u64,
    pub total_swap_kb: u64,
    pub total_cpu: f32,
    /// How many processes in this group only have approximate (RSS-based) memory.
    pub estimated_procs: usize,
    #[allow(dead_code)]
    pub main_pid: u32,
}

impl AppGroup {
    pub fn new(key: String, display_name: String, category: Category) -> Self {
        Self {
            key,
            display_name,
            category,
            processes: Vec::new(),
            total_pss_kb: 0,
            total_rss_kb: 0,
            total_uss_kb: 0,
            total_swap_kb: 0,
            total_cpu: 0.0,
            estimated_procs: 0,
            main_pid: 0,
        }
    }

    pub fn recompute_totals(&mut self) {
        self.total_pss_kb = self.processes.iter().map(|p| p.pss_kb).sum();
        self.total_rss_kb = self.processes.iter().map(|p| p.rss_kb).sum();
        self.total_uss_kb = self.processes.iter().map(|p| p.uss_kb).sum();
        self.total_swap_kb = self.processes.iter().map(|p| p.swap_kb).sum();
        self.total_cpu = self.processes.iter().map(|p| p.cpu_usage).sum();
        self.estimated_procs = self.processes.iter().filter(|p| p.pss_estimated).count();

        // Find representative main PID
        if let Some(first) = self.processes.iter().min_by_key(|p| p.pid) {
            self.main_pid = first.pid;
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct SystemMemoryInfo {
    pub total_kb: u64,
    pub available_kb: u64,
    pub free_kb: u64,
    pub buffers_cache_kb: u64,
    pub swap_total_kb: u64,
    pub swap_free_kb: u64,
    pub swap_used_kb: u64,
    pub total_pss_sum_kb: u64,
    pub total_rss_sum_kb: u64,
    /// Portion of `total_pss_sum_kb` that is an RSS-based approximation because
    /// smaps_rollup could not be read (foreign-owned processes).
    pub estimated_pss_kb: u64,
    pub estimated_proc_count: usize,
}

impl SystemMemoryInfo {
    pub fn used_kb(&self) -> u64 {
        self.total_kb.saturating_sub(self.available_kb)
    }
}
