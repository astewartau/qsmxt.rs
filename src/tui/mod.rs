mod app;
mod command;
mod ui;
mod widgets;

use std::io;

use crossterm::{
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};

use app::App;

fn restore_terminal(terminal: &mut Terminal<CrosstermBackend<io::Stdout>>) {
    let _ = disable_raw_mode();
    let _ = execute!(terminal.backend_mut(), LeaveAlternateScreen);
    let _ = terminal.show_cursor();
}

pub fn run_tui() -> crate::Result<()> {
    // Install panic hook to restore terminal on panic
    let original_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |panic| {
        let _ = disable_raw_mode();
        let _ = execute!(io::stdout(), LeaveAlternateScreen);
        original_hook(panic);
    }));

    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut app = App::new();

    // Main loop
    loop {
        // Rescan BIDS tree when on Filters tab if bids_dir changed
        if app.active_tab == 1 {
            let bids_dir = app.form.bids_dir.clone();
            app.filter_state.maybe_rescan(&bids_dir);
        }

        terminal.draw(|f| ui::draw(f, &app))?;

        if let crossterm::event::Event::Key(key) = crossterm::event::read()? {
            app.handle_key(key);
        }

        if app.should_quit {
            restore_terminal(&mut terminal);
            return Ok(());
        }

        if app.should_run {
            restore_terminal(&mut terminal);

            // Now that the terminal is restored, init the logger
            let args = command::build_run_args(&app)?;
            let log_level = if args.debug {
                log::LevelFilter::Debug
            } else {
                log::LevelFilter::Info
            };
            env_logger::Builder::new()
                .filter_level(log_level)
                .format_timestamp(None)
                .init();

            let cmd_string = command::build_command_string(&app);
            println!("\n  Running: {}\n", cmd_string);

            return crate::commands::run::execute(args);
        }
    }
}
