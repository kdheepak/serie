pub mod color;
pub mod git;
pub mod graph;
pub mod protocol;

mod app;
mod config;
mod event;
mod external;
mod macros;
mod view;
mod widget;

use std::{
    io::{stdout, Stdout},
    panic,
    path::Path,
};

use app::App;
use clap::{Parser, ValueEnum};
use ratatui::{
    backend::CrosstermBackend,
    crossterm::{
        execute,
        terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
    },
    Terminal,
};

/// Serie - A rich git commit graph in your terminal, like magic 📚
#[derive(Parser)]
#[command(version)]
struct Args {
    /// Image protocol to render graph
    #[arg(short, long, value_name = "TYPE", default_value = "iterm")]
    protocol: ImageProtocolType,

    /// Commit ordering algorithm
    #[arg(short, long, value_name = "TYPE", default_value = "chrono")]
    order: CommitOrderType,

    /// Do not use graph image cache
    #[arg(long, default_value = "false")]
    no_cache: bool,
}

#[derive(Debug, Clone, ValueEnum)]
enum ImageProtocolType {
    Iterm,
    Kitty,
}

impl From<ImageProtocolType> for protocol::ImageProtocol {
    fn from(protocol: ImageProtocolType) -> Self {
        match protocol {
            ImageProtocolType::Iterm => protocol::ImageProtocol::Iterm2,
            ImageProtocolType::Kitty => protocol::ImageProtocol::Kitty,
        }
    }
}

#[derive(Debug, Clone, ValueEnum)]
enum CommitOrderType {
    Chrono,
    Topo,
}

impl From<CommitOrderType> for graph::SortCommit {
    fn from(order: CommitOrderType) -> Self {
        match order {
            CommitOrderType::Chrono => graph::SortCommit::Chronological,
            CommitOrderType::Topo => graph::SortCommit::Topological,
        }
    }
}

fn setup() -> std::io::Result<Terminal<CrosstermBackend<Stdout>>> {
    enable_raw_mode()?;
    execute!(stdout(), EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout());
    Terminal::new(backend)
}

fn shutdown() -> std::io::Result<()> {
    execute!(stdout(), LeaveAlternateScreen)?;
    disable_raw_mode()?;
    Ok(())
}

fn initialize_panic_handler() {
    let original_hook = panic::take_hook();
    panic::set_hook(Box::new(move |panic_info| {
        shutdown().unwrap();
        original_hook(panic_info);
    }));
}

pub fn run() -> std::io::Result<()> {
    let args = Args::parse();
    let config = config::Config::load();

    let color_set = color::ColorSet::default();
    let image_protocol = args.protocol.into();

    let repository = git::Repository::load(Path::new("."));

    let graph_options = graph::CalcGraphOptions {
        sort: args.order.into(),
    };
    let graph = graph::calc_graph(&repository, graph_options);

    let graph_image_options = graph::GraphImageOptions::new(color_set.clone(), args.no_cache);
    let graph_image = graph::build_graph_image(&graph, graph_image_options);

    initialize_panic_handler();
    let mut terminal = setup()?;

    let (tx, rx) = event::init();

    let mut app = App::new(
        &repository,
        &graph,
        &graph_image,
        &config,
        &color_set,
        image_protocol,
        tx,
    );
    app.run(&mut terminal, rx)?;

    shutdown()?;
    Ok(())
}
