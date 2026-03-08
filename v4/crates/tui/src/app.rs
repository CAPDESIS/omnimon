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
