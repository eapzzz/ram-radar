#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Category {
    Apps,
    Desktop,
    Development,
    Background,
    System,
}

impl Category {
    pub fn title(&self) -> &'static str {
        match self {
            Category::Apps => "Aplikacje Użytkownika",
            Category::Desktop => "Środowisko Hyprland & Desktop",
            Category::Development => "Narzędzia Developerskie & Terminale",
            Category::Background => "Usługi Tła Użytkownika",
            Category::System => "System i Jądro",
        }
    }

    pub fn icon(&self) -> &'static str {
        match self {
            Category::Apps => "🌐",
            Category::Desktop => "🪟",
            Category::Development => "💻",
            Category::Background => "⚙️",
            Category::System => "🛡️",
        }
    }
}

#[derive(Debug, Clone)]
pub struct ProcessInfo {
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
    pub utime_stime: u64,
    pub cpu_usage: f32,
}

#[derive(Debug, Clone)]
pub struct AppGroup {
    pub key: String,
    pub display_name: String,
    pub category: Category,
    pub icon: &'static str,
    pub accent_color: [u8; 3],
    pub processes: Vec<ProcessInfo>,
    pub total_pss_kb: u64,
    pub total_rss_kb: u64,
    pub total_uss_kb: u64,
    pub total_swap_kb: u64,
    pub total_cpu: f32,
    pub main_pid: u32,
}

impl AppGroup {
    pub fn new(key: String, display_name: String, category: Category, icon: &'static str, accent_color: [u8; 3]) -> Self {
        Self {
            key,
            display_name,
            category,
            icon,
            accent_color,
            processes: Vec::new(),
            total_pss_kb: 0,
            total_rss_kb: 0,
            total_uss_kb: 0,
            total_swap_kb: 0,
            total_cpu: 0.0,
            main_pid: 0,
        }
    }

    pub fn recompute_totals(&mut self) {
        self.total_pss_kb = self.processes.iter().map(|p| p.pss_kb).sum();
        self.total_rss_kb = self.processes.iter().map(|p| p.rss_kb).sum();
        self.total_uss_kb = self.processes.iter().map(|p| p.uss_kb).sum();
        self.total_swap_kb = self.processes.iter().map(|p| p.swap_kb).sum();
        self.total_cpu = self.processes.iter().map(|p| p.cpu_usage).sum();
        
        // Find a representative main PID (lowest PID or first parent)
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
}

impl SystemMemoryInfo {
    pub fn used_kb(&self) -> u64 {
        self.total_kb.saturating_sub(self.available_kb)
    }

    pub fn used_percentage(&self) -> f32 {
        if self.total_kb == 0 {
            0.0
        } else {
            (self.used_kb() as f32 / self.total_kb as f32) * 100.0
        }
    }

    pub fn pss_percentage(&self) -> f32 {
        if self.total_kb == 0 {
            0.0
        } else {
            (self.total_pss_sum_kb as f32 / self.total_kb as f32) * 100.0
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SortColumn {
    PssRealisticRam,
    RssStandardRam,
    UssPrivateRam,
    Cpu,
    ProcessCount,
    Name,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SortDirection {
    Ascending,
    Descending,
}
