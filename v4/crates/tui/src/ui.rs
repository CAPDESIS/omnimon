//! Ratatui rendering — draws the full TUI layout each frame.
//!
//! Layout (vertical split):
//! ┌────────────────────────────────────────────┐
//! │  System Overview (gauges + summary)        │  3 lines
//! ├────────────────────────────────────────────┤
//! │  Process Table (scrollable)                │  ~65%
//! ├────────────────────────────────────────────┤
//! │  AI Chat Panel (messages + input)          │  ~35%
//! └────────────────────────────────────────────┘

use crate::app::{ActivePanel, App, ChatRole, SortColumn};
use crate::event;
use ratatui::prelude::*;
use ratatui::widgets::*;

/// Main draw function invoked every frame.
pub fn draw(f: &mut Frame, app: &mut App) {
    // Poll pending AI response each frame.
    event::poll_ai_response(app);

    let size = f.area();

    // Vertical layout: header (5), processes (flex), chat (flex).
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(5),
            Constraint::Percentage(55),
            Constraint::Percentage(40),
        ])
        .split(size);

    draw_header(f, app, chunks[0]);
    draw_process_table(f, app, chunks[1]);
    draw_chat_panel(f, app, chunks[2]);
}

// ─── Header: system gauges ───────────────────────────────────────────────────

fn draw_header(f: &mut Frame, app: &App, area: Rect) {
    let block = Block::default()
        .title(format!(" OmniMon v{} — TUI ", env!("CARGO_PKG_VERSION")))
        .title_alignment(Alignment::Center)
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan));

    let inner = block.inner(area);
    f.render_widget(block, area);

    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(25),
            Constraint::Percentage(25),
            Constraint::Percentage(25),
            Constraint::Percentage(25),
        ])
        .split(inner);

    // CPU gauge.
    let cpu_pct = app.state.cpu_usage_percent.clamp(0.0, 100.0) as u16;
    let cpu_gauge = Gauge::default()
        .block(Block::default().title("CPU"))
        .gauge_style(gauge_color(cpu_pct))
        .percent(cpu_pct)
        .label(format!("{:.1}%", app.state.cpu_usage_percent));
    f.render_widget(cpu_gauge, cols[0]);

    // Memory gauge.
    let mem_pct = if app.state.total_memory_bytes > 0 {
        ((app.state.used_memory_bytes as f64 / app.state.total_memory_bytes as f64) * 100.0) as u16
    } else {
        0
    };
    let mem_gauge = Gauge::default()
        .block(Block::default().title("MEM"))
        .gauge_style(gauge_color(mem_pct))
        .percent(mem_pct.min(100))
        .label(format!(
            "{} / {}",
            format_bytes(app.state.used_memory_bytes),
            format_bytes(app.state.total_memory_bytes)
        ));
    f.render_widget(mem_gauge, cols[1]);

    // Network.
    let net_text = format!(
        "↓ {}/s  ↑ {}/s",
        format_bytes(app.state.net_rx_bytes_per_sec),
        format_bytes(app.state.net_tx_bytes_per_sec),
    );
    let net_widget = Paragraph::new(net_text)
        .block(Block::default().title("NET"))
        .style(Style::default().fg(Color::White));
    f.render_widget(net_widget, cols[2]);

    // Process count + swap.
    let info_text = format!(
        "Procs: {}  Swap: {} MB",
        app.state.cached_process_info.len(),
        app.state.swap_used_mb,
    );
    let info_widget = Paragraph::new(info_text)
        .block(Block::default().title("SYS"))
        .style(Style::default().fg(Color::White));
    f.render_widget(info_widget, cols[3]);
}

fn gauge_color(pct: u16) -> Style {
    let color = if pct < 50 {
        Color::Green
    } else if pct < 80 {
        Color::Yellow
    } else {
        Color::Red
    };
    Style::default().fg(color).bg(Color::DarkGray)
}

// ─── Process table ───────────────────────────────────────────────────────────

fn draw_process_table(f: &mut Frame, app: &mut App, area: Rect) {
    let is_active = app.active_panel == ActivePanel::Processes;
    let border_color = if is_active {
        Color::Cyan
    } else {
        Color::DarkGray
    };

    let sort_indicator = match app.sort_col {
        SortColumn::Cpu => "CPU%",
        SortColumn::Memory => "MEM",
        SortColumn::Name => "NAME",
        SortColumn::Net => "NET",
        SortColumn::Energy => "NRG",
    };
    let sort_dir = if app.sort_asc { "↑" } else { "↓" };
    let title = format!(
        " Processes [{} {}] — s:sort r:reverse K:kill q:quit ",
        sort_indicator, sort_dir
    );

    let header = Row::new(vec![
        Cell::from("PID").style(
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        ),
        Cell::from("NAME").style(
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        ),
        Cell::from("CPU%").style(
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        ),
        Cell::from("MEMORY").style(
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        ),
        Cell::from("NET ↓↑").style(
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        ),
        Cell::from("ENERGY").style(
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        ),
    ])
    .height(1);

    let rows: Vec<Row> = app
        .sorted_processes
        .iter()
        .enumerate()
        .map(|(i, p)| {
            let style = if i == app.selected && is_active {
                Style::default().bg(Color::DarkGray).fg(Color::White)
            } else {
                Style::default()
            };

            let cpu_style = if p.cpu_pct > 80.0 {
                Style::default().fg(Color::Red)
            } else if p.cpu_pct > 30.0 {
                Style::default().fg(Color::Yellow)
            } else {
                Style::default().fg(Color::Green)
            };

            let net_total = p.net_rx_bytes_per_sec + p.net_tx_bytes_per_sec;
            let name = if p.name.len() > 24 {
                &p.name[..24]
            } else {
                &p.name
            };

            Row::new(vec![
                Cell::from(format!("{:>6}", p.pid)),
                Cell::from(name.to_string()),
                Cell::from(format!("{:>6.1}", p.cpu_pct)).style(cpu_style),
                Cell::from(format_bytes(p.memory_bytes)),
                Cell::from(format!("{}/s", format_bytes(net_total))),
                Cell::from(format!("{:.1}", p.energy_impact_score.unwrap_or(0.0))),
            ])
            .style(style)
        })
        .collect();

    // Adjust scroll offset to keep selection in view.
    let table_height = area.height.saturating_sub(4) as usize; // borders + header
    if app.selected < app.scroll_offset {
        app.scroll_offset = app.selected;
    } else if app.selected >= app.scroll_offset + table_height {
        app.scroll_offset = app.selected.saturating_sub(table_height) + 1;
    }

    let table = Table::new(
        rows,
        [
            Constraint::Length(7),
            Constraint::Min(16),
            Constraint::Length(8),
            Constraint::Length(10),
            Constraint::Length(12),
            Constraint::Length(8),
        ],
    )
    .header(header)
    .block(
        Block::default()
            .title(title)
            .borders(Borders::ALL)
            .border_style(Style::default().fg(border_color)),
    )
    .row_highlight_style(Style::default().add_modifier(Modifier::BOLD))
    .highlight_symbol("► ");

    let mut table_state = TableState::default().with_selected(Some(app.selected));
    table_state.select(Some(app.selected));
    *table_state.offset_mut() = app.scroll_offset;

    f.render_stateful_widget(table, area, &mut table_state);
}

// ─── AI Chat panel ───────────────────────────────────────────────────────────

fn draw_chat_panel(f: &mut Frame, app: &App, area: Rect) {
    let is_active = app.active_panel == ActivePanel::Chat;
    let border_color = if is_active {
        Color::Magenta
    } else {
        Color::DarkGray
    };

    let block = Block::default()
        .title(" AI Chat — Tab para cambiar, Enter para enviar ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(border_color));

    let inner = block.inner(area);
    f.render_widget(block, area);

    // Split: messages area + input line.
    let chat_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(1), Constraint::Length(1)])
        .split(inner);

    // Messages.
    let messages_height = chat_chunks[0].height as usize;
    let mut lines: Vec<Line> = Vec::new();
    for msg in &app.chat_messages {
        let (prefix, style) = match msg.role {
            ChatRole::User => ("► ", Style::default().fg(Color::Cyan)),
            ChatRole::Ai => ("AI: ", Style::default().fg(Color::Green)),
            ChatRole::System => ("sys: ", Style::default().fg(Color::DarkGray)),
        };

        // Wrap long messages into multiple lines.
        let max_width = chat_chunks[0].width.saturating_sub(2) as usize;
        let full = format!("{}{}", prefix, msg.text);
        for chunk in wrap_text(&full, max_width) {
            lines.push(Line::from(Span::styled(chunk, style)));
        }
    }

    // Auto-scroll to bottom.
    let skip = lines.len().saturating_sub(messages_height);
    let visible: Vec<Line> = lines.into_iter().skip(skip).collect();

    let messages_widget = Paragraph::new(visible).wrap(Wrap { trim: false });
    f.render_widget(messages_widget, chat_chunks[0]);

    // Input line.
    let input_style = if is_active {
        Style::default().fg(Color::White).bg(Color::DarkGray)
    } else {
        Style::default().fg(Color::DarkGray)
    };
    let cursor = if is_active && !app.chat_loading {
        "█"
    } else {
        ""
    };
    let input_text = format!("❯ {}{}", app.chat_input, cursor);
    let input_widget = Paragraph::new(input_text).style(input_style);
    f.render_widget(input_widget, chat_chunks[1]);
}

// ─── Helpers ─────────────────────────────────────────────────────────────────

fn format_bytes(bytes: u64) -> String {
    const KB: f64 = 1024.0;
    const MB: f64 = KB * 1024.0;
    const GB: f64 = MB * 1024.0;
    let b = bytes as f64;
    if b >= GB {
        format!("{:.1} GB", b / GB)
    } else if b >= MB {
        format!("{:.1} MB", b / MB)
    } else if b >= KB {
        format!("{:.0} KB", b / KB)
    } else {
        format!("{} B", bytes)
    }
}

fn wrap_text(text: &str, max_width: usize) -> Vec<String> {
    if max_width == 0 {
        return vec![text.to_string()];
    }
    let mut result = Vec::new();
    let mut current = String::new();
    for ch in text.chars() {
        if ch == '\n' {
            result.push(std::mem::take(&mut current));
            continue;
        }
        current.push(ch);
        if current.len() >= max_width {
            result.push(std::mem::take(&mut current));
        }
    }
    if !current.is_empty() {
        result.push(current);
    }
    if result.is_empty() {
        result.push(String::new());
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::{ActivePanel, App, ChatMessage, ChatRole};

    #[test]
    fn format_bytes_units() {
        assert_eq!(format_bytes(0), "0 B");
        assert_eq!(format_bytes(512), "512 B");
        assert_eq!(format_bytes(1024), "1 KB");
        assert_eq!(format_bytes(1_048_576), "1.0 MB");
        assert_eq!(format_bytes(1_073_741_824), "1.0 GB");
    }

    #[test]
    fn wrap_text_basic() {
        let lines = wrap_text("hello world this is a test", 10);
        assert_eq!(lines, vec!["hello worl", "d this is ", "a test"]);
    }

    #[test]
    fn wrap_text_short() {
        let lines = wrap_text("hi", 80);
        assert_eq!(lines, vec!["hi"]);
    }

    #[test]
    fn wrap_text_newlines() {
        let lines = wrap_text("line1\nline2", 80);
        assert_eq!(lines, vec!["line1", "line2"]);
    }

    #[test]
    fn wrap_text_zero_width_returns_full() {
        let lines = wrap_text("abc", 0);
        assert_eq!(lines, vec!["abc"]);
    }

    #[test]
    fn gauge_color_thresholds() {
        assert_eq!(gauge_color(10).fg, Some(Color::Green));
        assert_eq!(gauge_color(60).fg, Some(Color::Yellow));
        assert_eq!(gauge_color(90).fg, Some(Color::Red));
    }

    #[test]
    fn draw_renders_on_test_backend() {
        let backend = ratatui::backend::TestBackend::new(120, 40);
        let mut terminal = Terminal::new(backend).unwrap();
        let mut app = App::new();
        app.state.total_memory_bytes = 8 * 1024 * 1024 * 1024;
        app.state.used_memory_bytes = 4 * 1024 * 1024 * 1024;
        app.state.cpu_usage_percent = 42.0;
        app.state.net_rx_bytes_per_sec = 1024;
        app.state.net_tx_bytes_per_sec = 2048;
        app.sorted_processes = vec![core::watcher::CachedProcessInfo {
            pid: 1,
            name: "omnimon".into(),
            group_name: "omnimon".into(),
            memory_bytes: 1024 * 1024,
            virtual_memory_bytes: 2 * 1024 * 1024,
            cpu_pct: 1.5,
            exec_name: "omnimon".into(),
            exe_path: None,
            bundle_id: None,
            disk_read_bytes: 0,
            disk_write_bytes: 0,
            net_rx_bytes_per_sec: 10,
            net_tx_bytes_per_sec: 20,
            energy_impact_score: Some(1.0),
            start_time: 0,
        }];
        app.chat_messages.push(ChatMessage {
            role: ChatRole::User,
            text: "hello coverage".into(),
        });
        app.chat_messages.push(ChatMessage {
            role: ChatRole::Ai,
            text: "response".into(),
        });
        app.chat_input = "typed".into();
        app.active_panel = ActivePanel::Chat;

        terminal.draw(|f| draw(f, &mut app)).unwrap();

        // Second draw with process panel active and alternate sort columns.
        app.active_panel = ActivePanel::Processes;
        for _ in 0..5 {
            app.next_sort();
            terminal.draw(|f| draw(f, &mut app)).unwrap();
        }
    }
}
