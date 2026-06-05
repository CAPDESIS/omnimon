//! Event loop: polls crossterm for keyboard input and refreshes the UI at 2 Hz.

use crate::app::{ActivePanel, App, ChatMessage, ChatRole};
use crate::ui;
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyModifiers};
use ratatui::prelude::*;
use std::io;
use std::time::{Duration, Instant};

/// Tick rate (500 ms → 2 refreshes/sec, matching watcher cadence).
const TICK_RATE: Duration = Duration::from_millis(500);

pub fn run_loop<B: Backend>(terminal: &mut Terminal<B>, app: &mut App) -> io::Result<()> {
    // Wait briefly for the watcher to produce its first snapshot.
    std::thread::sleep(Duration::from_millis(600));
    app.refresh();

    let mut last_tick = Instant::now();

    loop {
        terminal.draw(|f| ui::draw(f, app))?;

        let timeout = TICK_RATE.saturating_sub(last_tick.elapsed());
        if event::poll(timeout)? {
            if let Event::Key(key) = event::read()? {
                handle_key(app, key);
            }
        }

        if last_tick.elapsed() >= TICK_RATE {
            app.refresh();
            last_tick = Instant::now();
        }

        if app.should_quit {
            return Ok(());
        }
    }
}

fn handle_key(app: &mut App, key: KeyEvent) {
    // Global shortcuts.
    match key.code {
        KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            app.should_quit = true;
            return;
        }
        KeyCode::Tab => {
            app.active_panel = match app.active_panel {
                ActivePanel::Processes => ActivePanel::Chat,
                ActivePanel::Chat => ActivePanel::Processes,
            };
            return;
        }
        _ => {}
    }

    match app.active_panel {
        ActivePanel::Processes => handle_process_key(app, key),
        ActivePanel::Chat => handle_chat_key(app, key),
    }
}

fn handle_process_key(app: &mut App, key: KeyEvent) {
    match key.code {
        KeyCode::Char('q') | KeyCode::Esc => app.should_quit = true,
        KeyCode::Up | KeyCode::Char('k') => app.select_up(),
        KeyCode::Down | KeyCode::Char('j') => app.select_down(),
        KeyCode::PageUp => app.select_page_up(20),
        KeyCode::PageDown => app.select_page_down(20),
        KeyCode::Home => app.selected = 0,
        KeyCode::End => {
            app.selected = app.sorted_processes.len().saturating_sub(1);
        }
        KeyCode::Char('s') => app.next_sort(),
        KeyCode::Char('r') => app.toggle_sort_dir(),
        KeyCode::Char('K') => {
            // Kill selected process.
            if let Some(proc) = app.sorted_processes.get(app.selected) {
                let _ = core::killer::kill_process_safe(proc.pid as i32, &[]);
            }
        }
        _ => {}
    }
}

fn handle_chat_key(app: &mut App, key: KeyEvent) {
    match key.code {
        KeyCode::Esc => app.active_panel = ActivePanel::Processes,
        KeyCode::Char(c) if !app.chat_loading => {
            app.chat_input.push(c);
        }
        KeyCode::Backspace if !app.chat_loading => {
            app.chat_input.pop();
        }
        KeyCode::Enter => {
            if app.chat_loading || app.chat_input.trim().is_empty() {
                return;
            }
            let user_msg = app.chat_input.drain(..).collect::<String>();
            app.chat_messages.push(ChatMessage {
                role: ChatRole::User,
                text: user_msg.clone(),
            });

            // Dispatch AI request on a background thread.
            app.chat_loading = true;
            let messages_for_ai: Vec<(String, String)> = app
                .chat_messages
                .iter()
                .filter(|m| m.role == ChatRole::User || m.role == ChatRole::Ai)
                .map(|m| {
                    let role = match m.role {
                        ChatRole::User => "user",
                        ChatRole::Ai => "assistant",
                        ChatRole::System => "system",
                    };
                    (role.to_string(), m.text.clone())
                })
                .collect();

            let system_prompt =
                core::ai::build_chat_system_prompt(&core::watcher::get_cached_state());

            // Try to get an Ollama key (local model — no API key needed).
            // Fall back to trying other providers.
            let (provider, model, api_key) = resolve_ai_config();

            // We use a simple blocking approach in a spawned thread, then
            // push the result directly. The event loop will pick it up
            // on the next tick since App is not Send (single-threaded TUI).
            // We use a channel to safely transfer the response.
            let (tx, rx) = std::sync::mpsc::channel::<String>();

            std::thread::spawn(move || {
                let rt = tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build();
                let response = match rt {
                    Ok(rt) => rt
                        .block_on(core::ai::chat_with_tools(
                            provider,
                            &model,
                            &api_key,
                            &messages_for_ai,
                            &system_prompt,
                        ))
                        .map(|(text, _tool)| text)
                        .unwrap_or_else(|e| format!("Error: {}", e)),
                    Err(e) => format!("Runtime error: {}", e),
                };
                let _ = tx.send(response);
            });

            // Store the receiver so we can poll it on future ticks.
            // We stash it via a small trick: push a placeholder and keep
            // the rx in a thread-local. Since our TUI is single-threaded
            // this is safe.
            app.chat_messages.push(ChatMessage {
                role: ChatRole::Ai,
                text: "Pensando...".into(),
            });

            // Spin-free poll: we'll check the channel each render tick.
            PENDING_AI_RX.with(|cell| {
                *cell.borrow_mut() = Some(rx);
            });
        }
        _ => {}
    }
}

thread_local! {
    static PENDING_AI_RX: std::cell::RefCell<Option<std::sync::mpsc::Receiver<String>>> =
        const { std::cell::RefCell::new(None) };
}

/// Called from the event loop each tick to check if the AI has responded.
pub fn poll_ai_response(app: &mut App) {
    PENDING_AI_RX.with(|cell| {
        let mut slot = cell.borrow_mut();
        if let Some(rx) = slot.as_ref() {
            if let Ok(response) = rx.try_recv() {
                // Replace the placeholder "Pensando..." message.
                if let Some(last) = app.chat_messages.last_mut() {
                    if last.role == ChatRole::Ai && last.text == "Pensando..." {
                        last.text = response;
                    }
                }
                app.chat_loading = false;
                *slot = None;
            }
        }
    });
}

/// Resolve the best available AI provider + model + API key.
fn resolve_ai_config() -> (core::ai::AiProvider, String, String) {
    // Prefer Ollama (local, no key required).
    if let Ok(key) = core::ai::get_api_key(core::ai::AiProvider::Ollama) {
        return (core::ai::AiProvider::Ollama, "llama3.2".into(), key);
    }
    // Then Anthropic.
    if let Ok(key) = core::ai::get_api_key(core::ai::AiProvider::Anthropic) {
        return (
            core::ai::AiProvider::Anthropic,
            "claude-haiku-4-5-20251001".into(),
            key,
        );
    }
    // Then OpenRouter (free tier).
    if let Ok(key) = core::ai::get_api_key(core::ai::AiProvider::OpenRouter) {
        return (
            core::ai::AiProvider::OpenRouter,
            "meta-llama/llama-3.2-3b-instruct:free".into(),
            key,
        );
    }
    // Then OpenAI.
    if let Ok(key) = core::ai::get_api_key(core::ai::AiProvider::OpenAI) {
        return (core::ai::AiProvider::OpenAI, "gpt-4o-mini".into(), key);
    }
    // Fallback: Ollama with empty key (works for local deployments).
    (
        core::ai::AiProvider::Ollama,
        "llama3.2".into(),
        String::new(),
    )
}
