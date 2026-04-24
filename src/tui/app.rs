use crossterm::event::{KeyCode, KeyEvent};

pub const TAB_NAMES: [&str; 5] = [
    "Input/Output",
    "Filters",
    "Algorithms",
    "Parameters",
    "Execution",
];

#[derive(Clone)]
pub enum FieldKind {
    Text,
    Select { options: Vec<&'static str> },
    Checkbox,
}

#[derive(Clone)]
pub struct FieldDef {
    pub label: &'static str,
    pub kind: FieldKind,
    pub help: &'static str,
}

pub struct App {
    pub active_tab: usize,
    pub active_field: usize,
    pub editing: bool,
    pub cursor_pos: usize,
    pub form: RunForm,
    pub should_quit: bool,
    pub should_run: bool,
    pub tab_fields: Vec<Vec<FieldDef>>,
}

pub struct RunForm {
    // Tab 0: Input/Output
    pub bids_dir: String,
    pub output_dir: String,
    pub preset: usize,
    pub config_file: String,

    // Tab 1: Filters
    pub subjects: String,
    pub sessions: String,
    pub acquisitions: String,
    pub runs_filter: String,
    pub num_echoes: String,

    // Tab 2: Algorithms
    pub qsm_algorithm: usize,
    pub unwrapping_algorithm: usize,
    pub bf_algorithm: usize,
    pub masking_algorithm: usize,
    pub masking_input: usize,

    // Tab 3: Parameters
    pub combine_phase: bool,
    pub bet_fractional_intensity: String,
    pub mask_erosions: String,
    pub rts_delta: String,
    pub rts_mu: String,
    pub rts_tol: String,
    pub tgv_iterations: String,
    pub tgv_erosions: String,
    pub tv_lambda: String,
    pub tkd_threshold: String,
    pub obliquity_threshold: String,

    // Mask pipeline (empty = use legacy masking)
    pub mask_ops: Vec<crate::pipeline::config::MaskOp>,
    #[allow(dead_code)]
    pub mask_ops_selected: usize,

    // Tab 4: Execution
    pub do_swi: bool,
    pub do_t2starmap: bool,
    pub do_r2starmap: bool,
    pub inhomogeneity_correction: bool,
    pub dry_run: bool,
    pub debug: bool,
    pub n_procs: String,
}

impl App {
    pub fn new() -> Self {
        let tab_fields = vec![
            // Tab 0: Input/Output
            vec![
                FieldDef {
                    label: "BIDS Directory",
                    kind: FieldKind::Text,
                    help: "Path to input BIDS dataset (required)",
                },
                FieldDef {
                    label: "Output Directory",
                    kind: FieldKind::Text,
                    help: "Path for output derivatives (required)",
                },
                FieldDef {
                    label: "Preset",
                    kind: FieldKind::Select {
                        options: vec!["(none)", "gre", "epi", "bet", "fast", "body"],
                    },
                    help: "Premade pipeline configuration preset",
                },
                FieldDef {
                    label: "Config File",
                    kind: FieldKind::Text,
                    help: "Custom TOML config file (overrides preset)",
                },
            ],
            // Tab 1: Filters
            vec![
                FieldDef {
                    label: "Subjects",
                    kind: FieldKind::Text,
                    help: "Space-separated subject IDs (e.g. sub-01 sub-02)",
                },
                FieldDef {
                    label: "Sessions",
                    kind: FieldKind::Text,
                    help: "Space-separated session IDs",
                },
                FieldDef {
                    label: "Acquisitions",
                    kind: FieldKind::Text,
                    help: "Space-separated acquisition labels",
                },
                FieldDef {
                    label: "Runs",
                    kind: FieldKind::Text,
                    help: "Space-separated run labels",
                },
                FieldDef {
                    label: "Num Echoes",
                    kind: FieldKind::Text,
                    help: "Limit number of echoes to process",
                },
            ],
            // Tab 2: Algorithms
            vec![
                FieldDef {
                    label: "QSM Algorithm",
                    kind: FieldKind::Select {
                        options: vec!["rts", "tv", "tkd", "tgv"],
                    },
                    help: "Dipole inversion algorithm",
                },
                FieldDef {
                    label: "Unwrapping",
                    kind: FieldKind::Select {
                        options: vec!["romeo", "laplacian"],
                    },
                    help: "Phase unwrapping algorithm",
                },
                FieldDef {
                    label: "BG Removal",
                    kind: FieldKind::Select {
                        options: vec!["vsharp", "pdf", "lbv", "ismv"],
                    },
                    help: "Background field removal algorithm",
                },
                FieldDef {
                    label: "Masking",
                    kind: FieldKind::Select {
                        options: vec!["bet", "threshold"],
                    },
                    help: "Masking algorithm (BET or threshold-based)",
                },
                FieldDef {
                    label: "Mask Input",
                    kind: FieldKind::Select {
                        options: vec!["magnitude-first", "magnitude", "magnitude-last", "phase-quality"],
                    },
                    help: "Masking input: magnitude (RSS combined), first/last echo, or ROMEO quality map",
                },
            ],
            // Tab 3: Parameters
            vec![
                FieldDef {
                    label: "Combine Phase",
                    kind: FieldKind::Checkbox,
                    help: "Combine multi-echo phase data",
                },
                FieldDef {
                    label: "BET Frac. Intensity",
                    kind: FieldKind::Text,
                    help: "BET fractional intensity threshold (0.0-1.0)",
                },
                FieldDef {
                    label: "Mask Erosions",
                    kind: FieldKind::Text,
                    help: "Space-separated erosion iterations (e.g. 2 3)",
                },
                FieldDef {
                    label: "RTS Delta",
                    kind: FieldKind::Text,
                    help: "RTS delta parameter (default: 0.15)",
                },
                FieldDef {
                    label: "RTS Mu",
                    kind: FieldKind::Text,
                    help: "RTS mu parameter (default: 1e5)",
                },
                FieldDef {
                    label: "RTS Tolerance",
                    kind: FieldKind::Text,
                    help: "RTS convergence tolerance (default: 1e-4)",
                },
                FieldDef {
                    label: "TGV Iterations",
                    kind: FieldKind::Text,
                    help: "TGV iteration count (default: 1000)",
                },
                FieldDef {
                    label: "TGV Erosions",
                    kind: FieldKind::Text,
                    help: "TGV mask erosion iterations (default: 3)",
                },
                FieldDef {
                    label: "TV Lambda",
                    kind: FieldKind::Text,
                    help: "TV regularisation lambda (default: 1e-3)",
                },
                FieldDef {
                    label: "TKD Threshold",
                    kind: FieldKind::Text,
                    help: "TKD k-space threshold (default: 0.15)",
                },
                FieldDef {
                    label: "Obliquity Threshold",
                    kind: FieldKind::Text,
                    help: "Resample to axial if obliquity exceeds degrees (-1 = disabled)",
                },
            ],
            // Tab 4: Execution
            vec![
                FieldDef {
                    label: "Compute SWI",
                    kind: FieldKind::Checkbox,
                    help: "Also compute susceptibility-weighted images",
                },
                FieldDef {
                    label: "Compute T2* Map",
                    kind: FieldKind::Checkbox,
                    help: "Compute T2* relaxation map (requires 3+ echoes with magnitude)",
                },
                FieldDef {
                    label: "Compute R2* Map",
                    kind: FieldKind::Checkbox,
                    help: "Compute R2* decay rate map (requires 3+ echoes with magnitude)",
                },
                FieldDef {
                    label: "Inhomogeneity Correction",
                    kind: FieldKind::Checkbox,
                    help: "Apply B1 field correction to magnitude before masking",
                },
                FieldDef {
                    label: "Dry Run",
                    kind: FieldKind::Checkbox,
                    help: "Print processing plan without executing",
                },
                FieldDef {
                    label: "Debug Logging",
                    kind: FieldKind::Checkbox,
                    help: "Enable verbose debug log output",
                },
                FieldDef {
                    label: "Num Processes",
                    kind: FieldKind::Text,
                    help: "Number of parallel threads (empty = auto)",
                },
            ],
        ];

        App {
            active_tab: 0,
            active_field: 0,
            editing: false,
            cursor_pos: 0,
            form: RunForm::default(),
            should_quit: false,
            should_run: false,
            tab_fields,
        }
    }

    pub fn field_count(&self) -> usize {
        self.tab_fields[self.active_tab].len()
    }

    pub fn current_field(&self) -> &FieldDef {
        &self.tab_fields[self.active_tab][self.active_field]
    }

    pub fn handle_key(&mut self, key: KeyEvent) {
        if self.editing {
            self.handle_editing_key(key);
            return;
        }

        match key.code {
            KeyCode::Char('q') | KeyCode::Esc => self.should_quit = true,

            // Tab switching
            KeyCode::Char(c @ '1'..='5') => {
                self.active_tab = (c as usize) - ('1' as usize);
                self.active_field = 0;
            }
            KeyCode::Tab => {
                self.active_tab = (self.active_tab + 1) % TAB_NAMES.len();
                self.active_field = 0;
            }
            KeyCode::BackTab => {
                self.active_tab = (self.active_tab + TAB_NAMES.len() - 1) % TAB_NAMES.len();
                self.active_field = 0;
            }

            // Field navigation
            KeyCode::Up | KeyCode::Char('k')
                if self.active_field > 0 => {
                    self.active_field -= 1;
                }
            KeyCode::Down | KeyCode::Char('j') => {
                let max = self.field_count().saturating_sub(1);
                if self.active_field < max {
                    self.active_field += 1;
                }
            }

            // Field interaction
            KeyCode::Enter | KeyCode::Char(' ') => self.interact_field(),
            KeyCode::Left => self.adjust_select(-1),
            KeyCode::Right => self.adjust_select(1),

            // Run
            KeyCode::F(5) => self.should_run = true,

            _ => {}
        }
    }

    fn handle_editing_key(&mut self, key: KeyEvent) {
        let mut cursor = self.cursor_pos;

        match key.code {
            KeyCode::Esc | KeyCode::Enter => {
                self.editing = false;
                return;
            }
            KeyCode::Char(c) => {
                self.text_value_mut().insert(cursor, c);
                cursor += 1;
            }
            KeyCode::Backspace
                if cursor > 0 => {
                    cursor -= 1;
                    self.text_value_mut().remove(cursor);
                }
            KeyCode::Delete => {
                let len = self.text_value().len();
                if cursor < len {
                    self.text_value_mut().remove(cursor);
                }
            }
            KeyCode::Left => {
                cursor = cursor.saturating_sub(1);
            }
            KeyCode::Right => {
                let len = self.text_value().len();
                if cursor < len {
                    cursor += 1;
                }
            }
            KeyCode::Home => cursor = 0,
            KeyCode::End => cursor = self.text_value().len(),
            _ => {}
        }

        self.cursor_pos = cursor;
    }

    fn interact_field(&mut self) {
        match &self.current_field().kind {
            FieldKind::Text => {
                self.editing = true;
                self.cursor_pos = self.text_value().len();
            }
            FieldKind::Checkbox => self.toggle_checkbox(),
            FieldKind::Select { options } => {
                let n = options.len();
                let val = self.select_value();
                self.set_select_value((val + 1) % n);
            }
        }
    }

    fn adjust_select(&mut self, delta: isize) {
        if let FieldKind::Select { options } = &self.current_field().kind {
            let n = options.len() as isize;
            let val = self.select_value() as isize;
            let new_val = (val + delta).rem_euclid(n) as usize;
            self.set_select_value(new_val);
        }
    }

    // --- Field value accessors ---

    pub fn text_value(&self) -> &str {
        match (self.active_tab, self.active_field) {
            (0, 0) => &self.form.bids_dir,
            (0, 1) => &self.form.output_dir,
            (0, 3) => &self.form.config_file,
            (1, 0) => &self.form.subjects,
            (1, 1) => &self.form.sessions,
            (1, 2) => &self.form.acquisitions,
            (1, 3) => &self.form.runs_filter,
            (1, 4) => &self.form.num_echoes,
            (3, 1) => &self.form.bet_fractional_intensity,
            (3, 2) => &self.form.mask_erosions,
            (3, 3) => &self.form.rts_delta,
            (3, 4) => &self.form.rts_mu,
            (3, 5) => &self.form.rts_tol,
            (3, 6) => &self.form.tgv_iterations,
            (3, 7) => &self.form.tgv_erosions,
            (3, 8) => &self.form.tv_lambda,
            (3, 9) => &self.form.tkd_threshold,
            (4, 6) => &self.form.n_procs,
            _ => "",
        }
    }

    fn text_value_mut(&mut self) -> &mut String {
        match (self.active_tab, self.active_field) {
            (0, 0) => &mut self.form.bids_dir,
            (0, 1) => &mut self.form.output_dir,
            (0, 3) => &mut self.form.config_file,
            (1, 0) => &mut self.form.subjects,
            (1, 1) => &mut self.form.sessions,
            (1, 2) => &mut self.form.acquisitions,
            (1, 3) => &mut self.form.runs_filter,
            (1, 4) => &mut self.form.num_echoes,
            (3, 1) => &mut self.form.bet_fractional_intensity,
            (3, 2) => &mut self.form.mask_erosions,
            (3, 3) => &mut self.form.rts_delta,
            (3, 4) => &mut self.form.rts_mu,
            (3, 5) => &mut self.form.rts_tol,
            (3, 6) => &mut self.form.tgv_iterations,
            (3, 7) => &mut self.form.tgv_erosions,
            (3, 8) => &mut self.form.tv_lambda,
            (3, 9) => &mut self.form.tkd_threshold,
            (3, 10) => &mut self.form.obliquity_threshold,
            (4, 6) => &mut self.form.n_procs,
            _ => unreachable!("text_value_mut called on non-text field"),
        }
    }

    pub fn select_value(&self) -> usize {
        match (self.active_tab, self.active_field) {
            (0, 2) => self.form.preset,
            (2, 0) => self.form.qsm_algorithm,
            (2, 1) => self.form.unwrapping_algorithm,
            (2, 2) => self.form.bf_algorithm,
            (2, 3) => self.form.masking_algorithm,
            (2, 4) => self.form.masking_input,
            _ => 0,
        }
    }

    fn set_select_value(&mut self, val: usize) {
        match (self.active_tab, self.active_field) {
            (0, 2) => self.form.preset = val,
            (2, 0) => self.form.qsm_algorithm = val,
            (2, 1) => self.form.unwrapping_algorithm = val,
            (2, 2) => self.form.bf_algorithm = val,
            (2, 3) => self.form.masking_algorithm = val,
            (2, 4) => self.form.masking_input = val,
            _ => {}
        }
    }

    #[allow(dead_code)]
    fn checkbox_value(&self) -> bool {
        match (self.active_tab, self.active_field) {
            (3, 0) => self.form.combine_phase,
            (4, 0) => self.form.do_swi,
            (4, 1) => self.form.do_t2starmap,
            (4, 2) => self.form.do_r2starmap,
            (4, 3) => self.form.inhomogeneity_correction,
            (4, 4) => self.form.dry_run,
            (4, 5) => self.form.debug,
            _ => false,
        }
    }

    fn toggle_checkbox(&mut self) {
        match (self.active_tab, self.active_field) {
            (3, 0) => self.form.combine_phase = !self.form.combine_phase,
            (4, 0) => self.form.do_swi = !self.form.do_swi,
            (4, 1) => self.form.do_t2starmap = !self.form.do_t2starmap,
            (4, 2) => self.form.do_r2starmap = !self.form.do_r2starmap,
            (4, 3) => self.form.inhomogeneity_correction = !self.form.inhomogeneity_correction,
            (4, 4) => self.form.dry_run = !self.form.dry_run,
            (4, 5) => self.form.debug = !self.form.debug,
            _ => {}
        }
    }

    // Generalized accessors for rendering arbitrary (tab, field) pairs
    pub fn get_text_value(&self, tab: usize, field: usize) -> &str {
        match (tab, field) {
            (0, 0) => &self.form.bids_dir,
            (0, 1) => &self.form.output_dir,
            (0, 3) => &self.form.config_file,
            (1, 0) => &self.form.subjects,
            (1, 1) => &self.form.sessions,
            (1, 2) => &self.form.acquisitions,
            (1, 3) => &self.form.runs_filter,
            (1, 4) => &self.form.num_echoes,
            (3, 1) => &self.form.bet_fractional_intensity,
            (3, 2) => &self.form.mask_erosions,
            (3, 3) => &self.form.rts_delta,
            (3, 4) => &self.form.rts_mu,
            (3, 5) => &self.form.rts_tol,
            (3, 6) => &self.form.tgv_iterations,
            (3, 7) => &self.form.tgv_erosions,
            (3, 8) => &self.form.tv_lambda,
            (3, 9) => &self.form.tkd_threshold,
            (3, 10) => &self.form.obliquity_threshold,
            (4, 6) => &self.form.n_procs,
            _ => "",
        }
    }

    pub fn get_select_value(&self, tab: usize, field: usize) -> usize {
        match (tab, field) {
            (0, 2) => self.form.preset,
            (2, 0) => self.form.qsm_algorithm,
            (2, 1) => self.form.unwrapping_algorithm,
            (2, 2) => self.form.bf_algorithm,
            (2, 3) => self.form.masking_algorithm,
            (2, 4) => self.form.masking_input,
            _ => 0,
        }
    }

    pub fn get_checkbox_value(&self, tab: usize, field: usize) -> bool {
        match (tab, field) {
            (3, 0) => self.form.combine_phase,
            (4, 0) => self.form.do_swi,
            (4, 1) => self.form.do_t2starmap,
            (4, 2) => self.form.do_r2starmap,
            (4, 3) => self.form.inhomogeneity_correction,
            (4, 4) => self.form.dry_run,
            (4, 5) => self.form.debug,
            _ => false,
        }
    }
}

impl Default for RunForm {
    fn default() -> Self {
        Self {
            bids_dir: String::new(),
            output_dir: String::new(),
            preset: 0,
            config_file: String::new(),
            subjects: String::new(),
            sessions: String::new(),
            acquisitions: String::new(),
            runs_filter: String::new(),
            num_echoes: String::new(),
            qsm_algorithm: 0,
            unwrapping_algorithm: 0,
            bf_algorithm: 1, // pdf (GRE preset default)
            masking_algorithm: 1, // threshold (otsu)
            masking_input: 0,
            combine_phase: false,
            bet_fractional_intensity: String::new(),
            mask_erosions: String::new(),
            rts_delta: String::new(),
            rts_mu: String::new(),
            rts_tol: String::new(),
            tgv_iterations: String::new(),
            tgv_erosions: String::new(),
            tv_lambda: String::new(),
            tkd_threshold: String::new(),
            obliquity_threshold: String::new(),
            mask_ops: Vec::new(),
            mask_ops_selected: 0,
            do_swi: false,
            do_t2starmap: false,
            do_r2starmap: false,
            inhomogeneity_correction: false,
            dry_run: false,
            debug: false,
            n_procs: String::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

    fn key(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::NONE)
    }

    // --- App::new ---

    #[test]
    fn test_app_new_defaults() {
        let app = App::new();
        assert_eq!(app.active_tab, 0);
        assert_eq!(app.active_field, 0);
        assert!(!app.editing);
        assert!(!app.should_quit);
        assert!(!app.should_run);
        assert_eq!(app.tab_fields.len(), 5);
    }

    #[test]
    fn test_field_count_per_tab() {
        let mut app = App::new();
        assert_eq!(app.field_count(), 4); // Tab 0: Input/Output
        app.active_tab = 1;
        assert_eq!(app.field_count(), 5); // Tab 1: Filters
        app.active_tab = 2;
        assert_eq!(app.field_count(), 5); // Tab 2: Algorithms
        app.active_tab = 3;
        assert_eq!(app.field_count(), 11); // Tab 3: Parameters
        app.active_tab = 4;
        assert_eq!(app.field_count(), 7); // Tab 4: Execution
    }

    // --- Navigation ---

    #[test]
    fn test_quit_on_q() {
        let mut app = App::new();
        app.handle_key(key(KeyCode::Char('q')));
        assert!(app.should_quit);
    }

    #[test]
    fn test_quit_on_esc() {
        let mut app = App::new();
        app.handle_key(key(KeyCode::Esc));
        assert!(app.should_quit);
    }

    #[test]
    fn test_tab_switching_numbers() {
        let mut app = App::new();
        app.handle_key(key(KeyCode::Char('3')));
        assert_eq!(app.active_tab, 2);
        app.handle_key(key(KeyCode::Char('1')));
        assert_eq!(app.active_tab, 0);
        app.handle_key(key(KeyCode::Char('5')));
        assert_eq!(app.active_tab, 4);
    }

    #[test]
    fn test_tab_switching_tab_key() {
        let mut app = App::new();
        app.handle_key(key(KeyCode::Tab));
        assert_eq!(app.active_tab, 1);
        app.handle_key(key(KeyCode::Tab));
        assert_eq!(app.active_tab, 2);
        // Wraps around
        app.active_tab = 4;
        app.handle_key(key(KeyCode::Tab));
        assert_eq!(app.active_tab, 0);
    }

    #[test]
    fn test_tab_switching_backtab() {
        let mut app = App::new();
        app.handle_key(KeyEvent::new(KeyCode::BackTab, KeyModifiers::NONE));
        assert_eq!(app.active_tab, 4); // wraps from 0 to 4
    }

    #[test]
    fn test_tab_switch_resets_field() {
        let mut app = App::new();
        app.active_field = 2;
        app.handle_key(key(KeyCode::Tab));
        assert_eq!(app.active_field, 0);
    }

    #[test]
    fn test_field_navigation_down() {
        let mut app = App::new();
        app.handle_key(key(KeyCode::Down));
        assert_eq!(app.active_field, 1);
        app.handle_key(key(KeyCode::Char('j')));
        assert_eq!(app.active_field, 2);
    }

    #[test]
    fn test_field_navigation_up() {
        let mut app = App::new();
        app.active_field = 2;
        app.handle_key(key(KeyCode::Up));
        assert_eq!(app.active_field, 1);
        app.handle_key(key(KeyCode::Char('k')));
        assert_eq!(app.active_field, 0);
    }

    #[test]
    fn test_field_navigation_clamped() {
        let mut app = App::new();
        // Can't go below 0
        app.handle_key(key(KeyCode::Up));
        assert_eq!(app.active_field, 0);
        // Can't go past max
        app.active_field = app.field_count() - 1;
        let max = app.active_field;
        app.handle_key(key(KeyCode::Down));
        assert_eq!(app.active_field, max);
    }

    // --- Text editing ---

    #[test]
    fn test_enter_editing_text_field() {
        let mut app = App::new();
        // Tab 0, field 0 is BIDS Directory (Text)
        app.handle_key(key(KeyCode::Enter));
        assert!(app.editing);
    }

    #[test]
    fn test_type_characters() {
        let mut app = App::new();
        app.handle_key(key(KeyCode::Enter)); // enter editing
        app.handle_key(key(KeyCode::Char('/')));
        app.handle_key(key(KeyCode::Char('d')));
        app.handle_key(key(KeyCode::Char('a')));
        app.handle_key(key(KeyCode::Char('t')));
        app.handle_key(key(KeyCode::Char('a')));
        assert_eq!(app.form.bids_dir, "/data");
        assert_eq!(app.cursor_pos, 5);
    }

    #[test]
    fn test_backspace() {
        let mut app = App::new();
        app.form.bids_dir = "abc".to_string();
        app.handle_key(key(KeyCode::Enter)); // enter editing, cursor at end (3)
        app.handle_key(key(KeyCode::Backspace));
        assert_eq!(app.form.bids_dir, "ab");
        assert_eq!(app.cursor_pos, 2);
    }

    #[test]
    fn test_backspace_at_start_does_nothing() {
        let mut app = App::new();
        app.form.bids_dir = "x".to_string();
        app.editing = true;
        app.cursor_pos = 0;
        app.handle_key(key(KeyCode::Backspace));
        assert_eq!(app.form.bids_dir, "x");
    }

    #[test]
    fn test_delete_key() {
        let mut app = App::new();
        app.form.bids_dir = "abc".to_string();
        app.editing = true;
        app.cursor_pos = 0;
        app.handle_key(key(KeyCode::Delete));
        assert_eq!(app.form.bids_dir, "bc");
    }

    #[test]
    fn test_cursor_left_right() {
        let mut app = App::new();
        app.form.bids_dir = "abc".to_string();
        app.editing = true;
        app.cursor_pos = 2;
        app.handle_key(key(KeyCode::Left));
        assert_eq!(app.cursor_pos, 1);
        app.handle_key(key(KeyCode::Right));
        assert_eq!(app.cursor_pos, 2);
    }

    #[test]
    fn test_cursor_left_at_zero() {
        let mut app = App::new();
        app.editing = true;
        app.cursor_pos = 0;
        app.handle_key(key(KeyCode::Left));
        assert_eq!(app.cursor_pos, 0);
    }

    #[test]
    fn test_home_end_keys() {
        let mut app = App::new();
        app.form.bids_dir = "abcdef".to_string();
        app.editing = true;
        app.cursor_pos = 3;
        app.handle_key(key(KeyCode::Home));
        assert_eq!(app.cursor_pos, 0);
        app.handle_key(key(KeyCode::End));
        assert_eq!(app.cursor_pos, 6);
    }

    #[test]
    fn test_esc_exits_editing() {
        let mut app = App::new();
        app.editing = true;
        app.handle_key(key(KeyCode::Esc));
        assert!(!app.editing);
        // Should NOT trigger quit while editing
        assert!(!app.should_quit);
    }

    #[test]
    fn test_enter_exits_editing() {
        let mut app = App::new();
        app.editing = true;
        app.handle_key(key(KeyCode::Enter));
        assert!(!app.editing);
    }

    // --- Select fields ---

    #[test]
    fn test_select_left_right() {
        let mut app = App::new();
        // Tab 0, field 2 is Preset (Select)
        app.active_field = 2;
        assert_eq!(app.select_value(), 0);
        app.handle_key(key(KeyCode::Right));
        assert_eq!(app.form.preset, 1);
        app.handle_key(key(KeyCode::Left));
        assert_eq!(app.form.preset, 0);
    }

    #[test]
    fn test_select_wraps_around() {
        let mut app = App::new();
        app.active_field = 2; // Preset with 6 options
        app.handle_key(key(KeyCode::Left));
        assert_eq!(app.form.preset, 5); // wraps to last
    }

    #[test]
    fn test_select_enter_cycles() {
        let mut app = App::new();
        app.active_field = 2;
        app.handle_key(key(KeyCode::Enter));
        assert_eq!(app.form.preset, 1);
        // Not editing — selects don't enter edit mode
        assert!(!app.editing);
    }

    // --- Checkbox fields ---

    #[test]
    fn test_checkbox_toggle() {
        let mut app = App::new();
        app.active_tab = 4; // Execution tab
        app.active_field = 0; // do_swi
        assert!(!app.form.do_swi);
        app.handle_key(key(KeyCode::Enter));
        assert!(app.form.do_swi);
        app.handle_key(key(KeyCode::Char(' ')));
        assert!(!app.form.do_swi);
    }

    #[test]
    fn test_checkbox_all_fields() {
        let mut app = App::new();
        app.active_tab = 4;

        // Toggle each checkbox
        for field in 0..6 {
            app.active_field = field;
            app.handle_key(key(KeyCode::Enter));
        }
        assert!(app.form.do_swi);
        assert!(app.form.do_t2starmap);
        assert!(app.form.do_r2starmap);
        assert!(app.form.inhomogeneity_correction);
        assert!(app.form.dry_run);
        assert!(app.form.debug);
    }

    #[test]
    fn test_combine_phase_checkbox() {
        let mut app = App::new();
        app.active_tab = 3;
        app.active_field = 0;
        assert!(!app.form.combine_phase);
        app.handle_key(key(KeyCode::Enter));
        assert!(app.form.combine_phase);
    }

    // --- F5 triggers run ---

    #[test]
    fn test_f5_triggers_run() {
        let mut app = App::new();
        app.handle_key(key(KeyCode::F(5)));
        assert!(app.should_run);
    }

    // --- Algorithm select fields ---

    #[test]
    fn test_algorithm_selects() {
        let mut app = App::new();
        app.active_tab = 2;

        // QSM algorithm (field 0)
        app.active_field = 0;
        app.handle_key(key(KeyCode::Right));
        assert_eq!(app.form.qsm_algorithm, 1); // tv

        // Unwrapping (field 1)
        app.active_field = 1;
        app.handle_key(key(KeyCode::Right));
        assert_eq!(app.form.unwrapping_algorithm, 1); // laplacian

        // BG removal (field 2)
        app.active_field = 2;
        app.handle_key(key(KeyCode::Right));
        assert_eq!(app.form.bf_algorithm, 2); // default 1 (pdf) + 1 = 2 (lbv)

        // Masking algorithm (field 3)
        app.active_field = 3;
        app.handle_key(key(KeyCode::Right));
        // default is 1 (threshold), cycles to 0 (bet) since only 2 options
        assert_eq!(app.form.masking_algorithm, 0); // bet

        // Masking input (field 4)
        app.active_field = 4;
        app.handle_key(key(KeyCode::Right));
        assert_eq!(app.form.masking_input, 1); // magnitude
    }

    // --- Text value accessors for different tabs ---

    #[test]
    fn test_text_value_filters_tab() {
        let mut app = App::new();
        app.form.subjects = "sub-01 sub-02".to_string();
        app.active_tab = 1;
        app.active_field = 0;
        assert_eq!(app.text_value(), "sub-01 sub-02");
    }

    #[test]
    fn test_text_value_params_tab() {
        let mut app = App::new();
        app.form.rts_delta = "0.2".to_string();
        app.active_tab = 3;
        app.active_field = 3;
        assert_eq!(app.text_value(), "0.2");
    }

    #[test]
    fn test_text_value_n_procs() {
        let mut app = App::new();
        app.form.n_procs = "8".to_string();
        app.active_tab = 4;
        app.active_field = 6;
        assert_eq!(app.text_value(), "8");
    }

    #[test]
    fn test_text_value_unknown_returns_empty() {
        let mut app = App::new();
        app.active_tab = 2; // Algorithms tab - no text fields
        app.active_field = 0;
        assert_eq!(app.text_value(), "");
    }

    // --- get_ accessors (used by UI rendering) ---

    #[test]
    fn test_get_text_value_all_fields() {
        let app = App::new();
        // Should not panic for any valid (tab, field) combo
        for tab in 0..5 {
            for field in 0..12 {
                let _ = app.get_text_value(tab, field);
            }
        }
    }

    #[test]
    fn test_get_select_value_defaults() {
        let app = App::new();
        assert_eq!(app.get_select_value(0, 2), 0); // preset
        assert_eq!(app.get_select_value(2, 0), 0); // qsm_algorithm (rts)
        assert_eq!(app.get_select_value(2, 1), 0); // unwrapping (romeo)
        assert_eq!(app.get_select_value(2, 2), 1); // bf_algorithm (pdf, default index 1)
        assert_eq!(app.get_select_value(99, 99), 0); // unknown returns 0
    }

    #[test]
    fn test_get_checkbox_value_defaults() {
        let app = App::new();
        assert!(!app.get_checkbox_value(4, 0)); // do_swi
        assert!(!app.get_checkbox_value(4, 4)); // dry_run
        assert!(!app.get_checkbox_value(99, 99)); // unknown returns false
    }

    // --- checkbox_value (private, exercised for coverage) ---

    #[test]
    fn test_checkbox_value_accessor() {
        let mut app = App::new();
        app.form.do_swi = true;
        app.active_tab = 4;
        app.active_field = 0;
        assert!(app.checkbox_value());

        app.active_field = 1; // do_t2starmap
        assert!(!app.checkbox_value());

        // Unknown field returns false
        app.active_tab = 0;
        app.active_field = 0;
        assert!(!app.checkbox_value());
    }

    // --- RunForm default ---

    #[test]
    fn test_run_form_default() {
        let form = RunForm::default();
        assert!(form.bids_dir.is_empty());
        assert!(form.output_dir.is_empty());
        assert_eq!(form.preset, 0);
        assert_eq!(form.bf_algorithm, 1); // pdf
        assert_eq!(form.masking_algorithm, 1); // threshold
        assert!(!form.combine_phase);
        assert!(form.mask_ops.is_empty());
    }

    // --- Editing output_dir ---

    #[test]
    fn test_edit_output_dir() {
        let mut app = App::new();
        app.active_field = 1; // output_dir
        app.handle_key(key(KeyCode::Enter));
        app.handle_key(key(KeyCode::Char('/')));
        app.handle_key(key(KeyCode::Char('o')));
        app.handle_key(key(KeyCode::Char('u')));
        app.handle_key(key(KeyCode::Char('t')));
        assert_eq!(app.form.output_dir, "/out");
    }

    // --- Editing config_file ---

    #[test]
    fn test_edit_config_file() {
        let mut app = App::new();
        app.active_field = 3; // config_file
        app.handle_key(key(KeyCode::Enter));
        app.handle_key(key(KeyCode::Char('c')));
        assert_eq!(app.form.config_file, "c");
    }

    // --- Editing all parameter text fields ---

    #[test]
    fn test_edit_parameter_fields() {
        let mut app = App::new();
        app.active_tab = 3;

        // bet_fractional_intensity (field 1)
        app.active_field = 1;
        app.handle_key(key(KeyCode::Enter));
        app.handle_key(key(KeyCode::Char('5')));
        app.handle_key(key(KeyCode::Enter));
        assert_eq!(app.form.bet_fractional_intensity, "5");

        // mask_erosions (field 2)
        app.active_field = 2;
        app.handle_key(key(KeyCode::Enter));
        app.handle_key(key(KeyCode::Char('2')));
        app.handle_key(key(KeyCode::Enter));
        assert_eq!(app.form.mask_erosions, "2");

        // rts_mu (field 4)
        app.active_field = 4;
        app.handle_key(key(KeyCode::Enter));
        app.handle_key(key(KeyCode::Char('9')));
        app.handle_key(key(KeyCode::Enter));
        assert_eq!(app.form.rts_mu, "9");

        // rts_tol (field 5)
        app.active_field = 5;
        app.handle_key(key(KeyCode::Enter));
        app.handle_key(key(KeyCode::Char('1')));
        app.handle_key(key(KeyCode::Enter));
        assert_eq!(app.form.rts_tol, "1");

        // tgv_iterations (field 6)
        app.active_field = 6;
        app.handle_key(key(KeyCode::Enter));
        app.handle_key(key(KeyCode::Char('5')));
        app.handle_key(key(KeyCode::Enter));
        assert_eq!(app.form.tgv_iterations, "5");

        // tgv_erosions (field 7)
        app.active_field = 7;
        app.handle_key(key(KeyCode::Enter));
        app.handle_key(key(KeyCode::Char('3')));
        app.handle_key(key(KeyCode::Enter));
        assert_eq!(app.form.tgv_erosions, "3");

        // tv_lambda (field 8)
        app.active_field = 8;
        app.handle_key(key(KeyCode::Enter));
        app.handle_key(key(KeyCode::Char('7')));
        app.handle_key(key(KeyCode::Enter));
        assert_eq!(app.form.tv_lambda, "7");

        // tkd_threshold (field 9)
        app.active_field = 9;
        app.handle_key(key(KeyCode::Enter));
        app.handle_key(key(KeyCode::Char('4')));
        app.handle_key(key(KeyCode::Enter));
        assert_eq!(app.form.tkd_threshold, "4");

        // obliquity_threshold (field 10)
        app.active_field = 10;
        app.handle_key(key(KeyCode::Enter));
        app.handle_key(key(KeyCode::Char('5')));
        app.handle_key(key(KeyCode::Enter));
        assert_eq!(app.form.obliquity_threshold, "5");
    }

    // --- Editing filter fields ---

    #[test]
    fn test_edit_filter_fields() {
        let mut app = App::new();
        app.active_tab = 1;

        for (field, form_field) in [
            (0, "subjects"),
            (1, "sessions"),
            (2, "acquisitions"),
            (3, "runs_filter"),
            (4, "num_echoes"),
        ] {
            app.active_field = field;
            app.handle_key(key(KeyCode::Enter));
            app.handle_key(key(KeyCode::Char('x')));
            app.handle_key(key(KeyCode::Enter));
            let val = match form_field {
                "subjects" => &app.form.subjects,
                "sessions" => &app.form.sessions,
                "acquisitions" => &app.form.acquisitions,
                "runs_filter" => &app.form.runs_filter,
                "num_echoes" => &app.form.num_echoes,
                _ => unreachable!(),
            };
            assert_eq!(val, "x", "Field {} should be 'x'", form_field);
        }
    }

    // --- Right arrow on non-select does nothing ---

    #[test]
    fn test_left_right_on_text_field_does_nothing() {
        let mut app = App::new();
        app.active_field = 0; // Text field
        app.handle_key(key(KeyCode::Left));
        app.handle_key(key(KeyCode::Right));
        // No crash, no state change
        assert_eq!(app.active_field, 0);
    }
}
