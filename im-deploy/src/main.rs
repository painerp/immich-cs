mod config;
mod commands;
mod openstack;
mod tailscale;
mod tui;

use anyhow::Result;
use clap::{Parser, Subcommand};
use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind},
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    prelude::*,
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph},
};
use std::io;

#[derive(Parser)]
#[command(name = "im-deploy")]
#[command(about = "K3s cluster deployment and management tool", long_about = None)]
struct Cli {
    /// Automatically confirm prompts
    #[arg(short = 'y', long = "yes", global = true)]
    yes: bool,

    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Deploy the K3s cluster using Terraform/OpenTofu
    Deploy,
    /// Destroy the K3s cluster
    Destroy,
    /// SSH into a cluster server
    Ssh,
    /// Copy kubeconfig from the cluster to local directory
    CopyKubeconfig,
    /// Monitor cluster formation and readiness
    Monitor,
}

struct MainMenuSelector {
    commands: Vec<(&'static str, &'static str)>,
    state: ListState,
}

impl MainMenuSelector {
    fn new() -> Self {
        let mut state = ListState::default();
        state.select(Some(0));
        Self {
            commands: vec![
                ("Deploy", "Deploy the K3s cluster using Terraform/OpenTofu"),
                ("Destroy", "Destroy the K3s cluster"),
                ("SSH", "SSH into a cluster server"),
                ("Copy Kubeconfig", "Copy kubeconfig from the cluster to local directory"),
                ("Monitor", "Monitor cluster formation and readiness"),
            ],
            state,
        }
    }

    fn next(&mut self) {
        let i = match self.state.selected() {
            Some(i) => (i + 1) % self.commands.len(),
            None => 0,
        };
        self.state.select(Some(i));
    }

    fn previous(&mut self) {
        let i = match self.state.selected() {
            Some(i) => {
                if i == 0 {
                    self.commands.len() - 1
                } else {
                    i - 1
                }
            }
            None => 0,
        };
        self.state.select(Some(i));
    }

    fn get_selected(&self) -> Option<Commands> {
        self.state.selected().map(|i| match i {
            0 => Commands::Deploy,
            1 => Commands::Destroy,
            2 => Commands::Ssh,
            3 => Commands::CopyKubeconfig,
            4 => Commands::Monitor,
            _ => Commands::Deploy,
        })
    }
}

fn run_main_menu() -> Result<Option<Commands>> {
    enable_raw_mode()?;
    crossterm::execute!(io::stdout(), EnterAlternateScreen)?;

    let mut terminal = Terminal::new(CrosstermBackend::new(io::stdout()))?;
    let mut selector = MainMenuSelector::new();

    let result = loop {
        terminal.draw(|frame| {
            let area = frame.area();

            let items: Vec<ListItem> = selector
                .commands
                .iter()
                .map(|(name, desc)| {
                    ListItem::new(vec![
                        Line::from(Span::styled(*name, Style::default().fg(Color::Cyan).bold())),
                        Line::from(Span::styled(format!("  {}", desc), Style::default().fg(Color::Gray))),
                    ])
                })
                .collect();

            let list = List::new(items)
                .block(
                    Block::default()
                        .title("im-deploy - K3s Cluster Management")
                        .borders(Borders::ALL),
                )
                .highlight_style(Style::default().bg(Color::DarkGray))
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
                    KeyCode::Down | KeyCode::Char('j') => selector.next(),
                    KeyCode::Up | KeyCode::Char('k') => selector.previous(),
                    KeyCode::Enter => break selector.get_selected(),
                    _ => {}
                }
            }
        }
    };

    disable_raw_mode()?;
    crossterm::execute!(io::stdout(), LeaveAlternateScreen)?;

    Ok(result)
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    let command = match cli.command {
        Some(cmd) => cmd,
        None => {
            // No command provided, show interactive menu
            match run_main_menu()? {
                Some(cmd) => cmd,
                None => {
                    println!("Exiting.");
                    return Ok(());
                }
            }
        }
    };

    // Load configuration
    let config = config::load_config()?;

    match command {
        Commands::Deploy => commands::cmd_deploy(&config, cli.yes),
        Commands::Destroy => commands::cmd_destroy(&config, cli.yes),
        Commands::Ssh => commands::cmd_ssh(&config),
        Commands::CopyKubeconfig => commands::cmd_copy_kubeconfig(&config),
        Commands::Monitor => commands::cmd_monitor(&config),
    }
}

