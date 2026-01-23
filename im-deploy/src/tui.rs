use crate::domain::cluster::{CloudProvider, ServerInfo};
use crate::errors::Result;
use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind},
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    prelude::*,
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph},
};
use std::io;

pub struct ServerSelector {
    servers: Vec<ServerInfo>,
    state: ListState,
}

impl ServerSelector {
    fn new(servers: Vec<ServerInfo>) -> Self {
        let mut state = ListState::default();
        if !servers.is_empty() {
            state.select(Some(0));
        }
        Self { servers, state }
    }

    fn next(&mut self) {
        if self.servers.is_empty() {
            return;
        }
        let i = match self.state.selected() {
            Some(i) => (i + 1) % self.servers.len(),
            None => 0,
        };
        self.state.select(Some(i));
    }

    fn previous(&mut self) {
        if self.servers.is_empty() {
            return;
        }
        let i = match self.state.selected() {
            Some(i) => {
                if i == 0 {
                    self.servers.len() - 1
                } else {
                    i - 1
                }
            }
            None => 0,
        };
        self.state.select(Some(i));
    }

    fn get_selected(&self) -> Option<&ServerInfo> {
        self.state.selected().map(|i| &self.servers[i])
    }
}

pub struct CloudProviderSelector {
    providers: Vec<CloudProvider>,
    state: ListState,
}

impl CloudProviderSelector {
    fn new(providers: Vec<CloudProvider>) -> Self {
        let mut state = ListState::default();
        if !providers.is_empty() {
            state.select(Some(0));
        }
        Self { providers, state }
    }

    fn next(&mut self) {
        if self.providers.is_empty() {
            return;
        }
        let i = match self.state.selected() {
            Some(i) => (i + 1) % self.providers.len(),
            None => 0,
        };
        self.state.select(Some(i));
    }

    fn previous(&mut self) {
        if self.providers.is_empty() {
            return;
        }
        let i = match self.state.selected() {
            Some(i) => {
                if i == 0 {
                    self.providers.len() - 1
                } else {
                    i - 1
                }
            }
            None => 0,
        };
        self.state.select(Some(i));
    }

    fn get_selected(&self) -> Option<&CloudProvider> {
        self.state.selected().map(|i| &self.providers[i])
    }
}

pub fn run_server_selector(servers: Vec<ServerInfo>) -> Result<Option<ServerInfo>> {
    enable_raw_mode()?;
    crossterm::execute!(io::stdout(), EnterAlternateScreen)?;

    let mut terminal = Terminal::new(CrosstermBackend::new(io::stdout()))?;
    let mut selector = ServerSelector::new(servers);

    let result = loop {
        terminal.draw(|frame| {
            let area = frame.area();

            let items: Vec<ListItem> = selector
                .servers
                .iter()
                .map(|server| {
                    ListItem::new(format!("{} ({})", server.name, server.ip))
                })
                .collect();

            let list = List::new(items)
                .block(
                    Block::default()
                        .title("Select Server to SSH")
                        .borders(Borders::ALL),
                )
                .highlight_style(Style::default().fg(Color::Yellow))
                .highlight_symbol("> ");

            frame.render_stateful_widget(list, area, &mut selector.state);

            let help_text = "\nPress ↑/↓ to navigate, Enter to connect, Q to quit";
            let help_paragraph = Paragraph::new(help_text)
                .block(Block::default().borders(Borders::NONE));

            let help_area = Rect::new(area.x, area.bottom().saturating_sub(2), area.width, 2);
            frame.render_widget(help_paragraph, help_area);
        })?;

        if let Event::Key(key) = event::read()? {
            if key.kind == KeyEventKind::Press {
                match key.code {
                    KeyCode::Char('q') | KeyCode::Char('Q') => break None,
                    KeyCode::Down => selector.next(),
                    KeyCode::Up => selector.previous(),
                    KeyCode::Enter => break selector.get_selected().cloned(),
                    _ => {}
                }
            }
        }
    };

    disable_raw_mode()?;
    crossterm::execute!(io::stdout(), LeaveAlternateScreen)?;

    Ok(result)
}

pub fn run_cloud_provider_selector(providers: Vec<CloudProvider>) -> Result<Option<CloudProvider>> {
    enable_raw_mode()?;
    crossterm::execute!(io::stdout(), EnterAlternateScreen)?;

    let mut terminal = Terminal::new(CrosstermBackend::new(io::stdout()))?;
    let mut selector = CloudProviderSelector::new(providers);

    let result = loop {
        terminal.draw(|frame| {
            let area = frame.area();

            let items: Vec<ListItem> = selector
                .providers
                .iter()
                .map(|provider| {
                    let server_count = provider.server_count();
                    let agent_count = provider.agent_count();
                    ListItem::new(format!(
                        "{} ({} servers, {} agents)",
                        provider.name, server_count, agent_count
                    ))
                })
                .collect();

            let list = List::new(items)
                .block(
                    Block::default()
                        .title("Select Cloud Provider")
                        .borders(Borders::ALL),
                )
                .highlight_style(Style::default().fg(Color::Yellow))
                .highlight_symbol("> ");

            frame.render_stateful_widget(list, area, &mut selector.state);

            let help_text = "\nPress ↑/↓ to navigate, Enter to select, Q to quit";
            let help_paragraph = Paragraph::new(help_text)
                .block(Block::default().borders(Borders::NONE));

            let help_area = Rect::new(area.x, area.bottom().saturating_sub(2), area.width, 2);
            frame.render_widget(help_paragraph, help_area);
        })?;

        if let Event::Key(key) = event::read()? {
            if key.kind == KeyEventKind::Press {
                match key.code {
                    KeyCode::Char('q') | KeyCode::Char('Q') => break None,
                    KeyCode::Down => selector.next(),
                    KeyCode::Up => selector.previous(),
                    KeyCode::Enter => break selector.get_selected().cloned(),
                    _ => {}
                }
            }
        }
    };

    disable_raw_mode()?;
    crossterm::execute!(io::stdout(), LeaveAlternateScreen)?;

    Ok(result)
}

