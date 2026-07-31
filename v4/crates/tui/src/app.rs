//! Application state for the TUI.
//!
//! Keeps a lightweight snapshot of system metrics (refreshed each render tick)
//! plus UI state such as scroll position, active panel, and AI chat history.

use core::watcher::{CachedProcessInfo, SystemState};

/// Which panel currently has keyboard focus.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActivePanel {
    Processes,
    Chat,
}

/// Sorting column for the process table.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SortColumn {
    Cpu,
    Memory,
    Name,
    Net,
    Energy,
}

/// A single message in the AI chat panel.
#[derive(Debug, Clone)]
pub struct ChatMessage {
    pub role: ChatRole,
    pub text: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChatRole {
    User,
    Ai,
    System,
}

/// Top-level application state. All fields are cheaply updatable.
pub struct App {
    /// Latest system state snapshot from the watcher.
    pub state: SystemState,
    /// Sorted process list (derived from `state.cached_process_info`).
    pub sorted_processes: Vec<CachedProcessInfo>,
    /// Current sort column.
    pub sort_col: SortColumn,
    /// Sort ascending (true) or descending (false).
    pub sort_asc: bool,
    /// Selected row index in the process table.
    pub selected: usize,
    /// Scroll offset for the process table viewport.
    pub scroll_offset: usize,
    /// Currently focused panel.
    pub active_panel: ActivePanel,
    /// Chat history.
    pub chat_messages: Vec<ChatMessage>,
    /// Current chat input buffer.
    pub chat_input: String,
    /// Whether an AI request is in-flight.
    pub chat_loading: bool,
    /// Flag to signal exit.
    pub should_quit: bool,
}

impl App {
    pub fn new() -> Self {
        Self {
            state: SystemState::default(),
            sorted_processes: Vec::with_capacity(512),
            sort_col: SortColumn::Memory,
            sort_asc: false,
            selected: 0,
            scroll_offset: 0,
            active_panel: ActivePanel::Processes,
            chat_messages: vec![ChatMessage {
                role: ChatRole::System,
                text: "OmniMon AI Chat — escribe tu consulta y presiona Enter.".into(),
            }],
            chat_input: String::with_capacity(256),
            chat_loading: false,
            should_quit: false,
        }
    }

    /// Pull the latest watcher snapshot and rebuild the sorted process list.
    pub fn refresh(&mut self) {
        self.state = core::watcher::get_cached_state();

        self.sorted_processes.clear();
        self.sorted_processes
            .extend(self.state.cached_process_info.iter().cloned());

        let asc = self.sort_asc;
        match self.sort_col {
            SortColumn::Cpu => self.sorted_processes.sort_by(|a, b| {
                let cmp = a
                    .cpu_pct
                    .partial_cmp(&b.cpu_pct)
                    .unwrap_or(std::cmp::Ordering::Equal);
                if asc {
                    cmp
                } else {
                    cmp.reverse()
                }
            }),
            SortColumn::Memory => self.sorted_processes.sort_by(|a, b| {
                let cmp = a.memory_bytes.cmp(&b.memory_bytes);
                if asc {
                    cmp
                } else {
                    cmp.reverse()
                }
            }),
            SortColumn::Name => self.sorted_processes.sort_by(|a, b| {
                let cmp = a.name.to_lowercase().cmp(&b.name.to_lowercase());
                if asc {
                    cmp
                } else {
                    cmp.reverse()
                }
            }),
            SortColumn::Net => self.sorted_processes.sort_by(|a, b| {
                let a_net = a.net_rx_bytes_per_sec + a.net_tx_bytes_per_sec;
                let b_net = b.net_rx_bytes_per_sec + b.net_tx_bytes_per_sec;
                let cmp = a_net.cmp(&b_net);
                if asc {
                    cmp
                } else {
                    cmp.reverse()
                }
            }),
            SortColumn::Energy => self.sorted_processes.sort_by(|a, b| {
                let cmp = a
                    .energy_impact_score
                    .unwrap_or(0.0)
                    .partial_cmp(&b.energy_impact_score.unwrap_or(0.0))
                    .unwrap_or(std::cmp::Ordering::Equal);
                if asc {
                    cmp
                } else {
                    cmp.reverse()
                }
            }),
        }

        // Clamp selection.
        let len = self.sorted_processes.len();
        if len == 0 {
            self.selected = 0;
            self.scroll_offset = 0;
        } else if self.selected >= len {
            self.selected = len - 1;
        }
    }

    /// Cycle to the next sort column.
    pub fn next_sort(&mut self) {
        self.sort_col = match self.sort_col {
            SortColumn::Cpu => SortColumn::Memory,
            SortColumn::Memory => SortColumn::Name,
            SortColumn::Name => SortColumn::Net,
            SortColumn::Net => SortColumn::Energy,
            SortColumn::Energy => SortColumn::Cpu,
        };
        self.sort_asc = false;
    }

    /// Toggle sort direction.
    pub fn toggle_sort_dir(&mut self) {
        self.sort_asc = !self.sort_asc;
    }

    pub fn select_up(&mut self) {
        if self.selected > 0 {
            self.selected -= 1;
        }
    }

    pub fn select_down(&mut self) {
        if !self.sorted_processes.is_empty() && self.selected < self.sorted_processes.len() - 1 {
            self.selected += 1;
        }
    }

    pub fn select_page_up(&mut self, page: usize) {
        self.selected = self.selected.saturating_sub(page);
    }

    pub fn select_page_down(&mut self, page: usize) {
        let max = self.sorted_processes.len().saturating_sub(1);
        self.selected = (self.selected + page).min(max);
    }
}

#[cfg(test)]
#[allow(clippy::items_after_test_module)]
mod tests {
    use super::*;

    #[test]
    fn app_new_has_default_state() {
        let app = App::new();
        assert_eq!(app.sort_col, SortColumn::Memory);
        assert!(!app.sort_asc);
        assert_eq!(app.selected, 0);
        assert_eq!(app.active_panel, ActivePanel::Processes);
        assert!(!app.should_quit);
        assert_eq!(app.chat_messages.len(), 1);
    }

    #[test]
    fn next_sort_cycles_columns() {
        let mut app = App::new();
        assert_eq!(app.sort_col, SortColumn::Memory);
        app.next_sort();
        assert_eq!(app.sort_col, SortColumn::Name);
        app.next_sort();
        assert_eq!(app.sort_col, SortColumn::Net);
        app.next_sort();
        assert_eq!(app.sort_col, SortColumn::Energy);
        app.next_sort();
        assert_eq!(app.sort_col, SortColumn::Cpu);
        app.next_sort();
        assert_eq!(app.sort_col, SortColumn::Memory);
    }

    #[test]
    fn toggle_sort_dir_flips_flag() {
        let mut app = App::new();
        assert!(!app.sort_asc);
        app.toggle_sort_dir();
        assert!(app.sort_asc);
        app.toggle_sort_dir();
        assert!(!app.sort_asc);
    }

    #[test]
    fn select_up_does_not_go_below_zero() {
        let mut app = App::new();
        app.selected = 0;
        app.select_up();
        assert_eq!(app.selected, 0);
    }

    #[test]
    fn select_down_respects_bounds() {
        let mut app = App::new();
        app.selected = 0;
        // Empty list: can't go down
        app.select_down();
        assert_eq!(app.selected, 0);
    }

    #[test]
    fn select_page_up_saturates_to_zero() {
        let mut app = App::new();
        app.selected = 3;
        app.select_page_up(10);
        assert_eq!(app.selected, 0);
    }

    #[test]
    fn select_page_down_clamps_to_max() {
        let mut app = App::new();
        app.selected = 0;
        app.select_page_down(5);
        // Empty list: max is 0
        assert_eq!(app.selected, 0);
    }

    fn sample_processes_for_sort() -> Vec<CachedProcessInfo> {
        vec![
            CachedProcessInfo {
                pid: 1,
                name: "zeta".into(),
                exec_name: "zeta".into(),
                group_name: "z".into(),
                memory_bytes: 100,
                cpu_pct: 1.0,
                net_rx_bytes_per_sec: 5,
                net_tx_bytes_per_sec: 1,
                energy_impact_score: Some(1.0),
                ..Default::default()
            },
            CachedProcessInfo {
                pid: 2,
                name: "Alpha".into(),
                exec_name: "alpha".into(),
                group_name: "a".into(),
                memory_bytes: 500,
                cpu_pct: 9.0,
                net_rx_bytes_per_sec: 50,
                net_tx_bytes_per_sec: 10,
                energy_impact_score: Some(9.0),
                ..Default::default()
            },
            CachedProcessInfo {
                pid: 3,
                name: "mid".into(),
                exec_name: "mid".into(),
                group_name: "m".into(),
                memory_bytes: 250,
                cpu_pct: 4.0,
                net_rx_bytes_per_sec: 20,
                net_tx_bytes_per_sec: 5,
                energy_impact_score: Some(4.0),
                ..Default::default()
            },
        ]
    }

    fn assert_sorted<F>(procs: &[CachedProcessInfo], ascending: bool, key: F)
    where
        F: Fn(&CachedProcessInfo) -> u64,
    {
        for window in procs.windows(2) {
            let left = key(&window[0]);
            let right = key(&window[1]);
            if ascending {
                assert!(left <= right, "expected ascending order by key");
            } else {
                assert!(left >= right, "expected descending order by key");
            }
        }
    }

    #[test]
    fn refresh_sorts_by_each_column_and_clamps_selection() {
        core::watcher::start_watcher();
        std::thread::sleep(std::time::Duration::from_millis(700));

        let mut app = App::new();
        app.selected = 9999;

        app.sort_col = SortColumn::Memory;
        app.sort_asc = false;
        app.refresh();
        if !app.sorted_processes.is_empty() {
            assert_sorted(&app.sorted_processes, false, |p| p.memory_bytes);
            assert!(app.selected < app.sorted_processes.len());
        }

        app.sort_col = SortColumn::Cpu;
        app.sort_asc = true;
        app.refresh();
        if app.sorted_processes.len() >= 2 {
            assert_sorted(&app.sorted_processes, true, |p| p.cpu_pct as u64);
        }

        app.sort_col = SortColumn::Name;
        app.sort_asc = true;
        app.refresh();
        if app.sorted_processes.len() >= 2 {
            let names: Vec<String> = app
                .sorted_processes
                .iter()
                .map(|p| p.name.to_lowercase())
                .collect();
            for window in names.windows(2) {
                assert!(window[0] <= window[1]);
            }
        }

        app.sort_col = SortColumn::Net;
        app.sort_asc = false;
        app.refresh();
        if app.sorted_processes.len() >= 2 {
            assert_sorted(&app.sorted_processes, false, |p| {
                p.net_rx_bytes_per_sec + p.net_tx_bytes_per_sec
            });
        }

        app.sort_col = SortColumn::Energy;
        app.sort_asc = false;
        app.refresh();
        if app.sorted_processes.len() >= 2 {
            for window in app.sorted_processes.windows(2) {
                let left = window[0].energy_impact_score.unwrap_or(0.0);
                let right = window[1].energy_impact_score.unwrap_or(0.0);
                assert!(left >= right);
            }
        }
    }

    #[test]
    fn refresh_with_injected_processes_sorts_locally() {
        let mut app = App::new();
        app.state.cached_process_info = sample_processes_for_sort();
        app.sorted_processes = app.state.cached_process_info.clone();
        app.sort_col = SortColumn::Memory;
        app.sort_asc = false;

        app.sorted_processes.clear();
        app.sorted_processes
            .extend(app.state.cached_process_info.iter().cloned());
        app.sorted_processes
            .sort_by(|a, b| b.memory_bytes.cmp(&a.memory_bytes));

        assert_eq!(app.sorted_processes[0].memory_bytes, 500);
        assert_eq!(app.sorted_processes.last().unwrap().memory_bytes, 100);
    }

    #[test]
    fn app_navigation_with_prefilled_processes() {
        let mut app = App::new();
        app.sorted_processes = vec![
            CachedProcessInfo {
                pid: 1,
                name: "zeta".into(),
                group_name: "z".into(),
                memory_bytes: 100,
                cpu_pct: 1.0,
                net_rx_bytes_per_sec: 5,
                energy_impact_score: Some(1.0),
                ..Default::default()
            },
            CachedProcessInfo {
                pid: 2,
                name: "alpha".into(),
                group_name: "a".into(),
                memory_bytes: 500,
                cpu_pct: 9.0,
                net_rx_bytes_per_sec: 50,
                energy_impact_score: Some(9.0),
                ..Default::default()
            },
            CachedProcessInfo {
                pid: 3,
                name: "mid".into(),
                group_name: "m".into(),
                memory_bytes: 250,
                cpu_pct: 4.0,
                ..Default::default()
            },
        ];
        app.selected = 1;
        app.select_up();
        assert_eq!(app.selected, 0);
        app.select_down();
        assert_eq!(app.selected, 1);
        app.select_page_down(10);
        assert_eq!(app.selected, 2);
        app.select_page_up(10);
        assert_eq!(app.selected, 0);
        app.next_sort();
        assert_eq!(app.sort_col, SortColumn::Name);
    }
}
