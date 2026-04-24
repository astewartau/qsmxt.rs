use std::collections::HashSet;
use std::path::Path;

use crossterm::event::{KeyCode, KeyEvent};

use crate::bids::discovery::{self, BidsTree};

pub const TAB_NAMES: [&str; 5] = [
    "Input/Output",
    "Filters",
    "Algorithms",
    "Parameters",
    "Execution",
];

// ─── Filter tree state ───

/// What is focused in the flattened filter tree view.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FilterFocus {
    Pattern,
    TreeNode(usize), // index into the flattened visible node list
    NumEchoes,
}

/// A single visible row in the flattened tree (for navigation/rendering).
#[derive(Debug, Clone)]
pub enum TreeRow {
    Subject(usize),                 // index into tree.subjects
    Session(usize, usize),         // (subject_idx, session_idx)
    Run { sub: usize, ses: Option<usize>, run: usize }, // leaf run
}

/// Filter tab state: tree of BIDS runs with selection and navigation.
#[derive(Debug, Clone)]
pub struct FilterTreeState {
    pub tree: Option<BidsTree>,
    pub collapsed: HashSet<String>,
    pub focus: FilterFocus,
    pub pattern: String,
    pub pattern_editing: bool,
    pub pattern_cursor: usize,
    pub num_echoes: String,
    pub num_echoes_editing: bool,
    pub num_echoes_cursor: usize,
    pub scanned_bids_dir: Option<String>,
    pub scroll_offset: usize,
}

impl Default for FilterTreeState {
    fn default() -> Self {
        Self {
            tree: None,
            collapsed: HashSet::new(),
            focus: FilterFocus::Pattern,
            pattern: String::new(),
            pattern_editing: false,
            pattern_cursor: 0,
            num_echoes: String::new(),
            num_echoes_editing: false,
            num_echoes_cursor: 0,
            scanned_bids_dir: None,
            scroll_offset: 0,
        }
    }
}

impl FilterTreeState {
    /// Build the flattened list of visible tree rows (respecting collapsed state).
    pub fn visible_rows(&self) -> Vec<TreeRow> {
        let Some(ref tree) = self.tree else { return Vec::new() };
        let mut rows = Vec::new();
        for (si, sub) in tree.subjects.iter().enumerate() {
            rows.push(TreeRow::Subject(si));
            if self.collapsed.contains(&format!("sub-{}", sub.name)) {
                continue;
            }
            // Direct runs (no session)
            for ri in 0..sub.runs.len() {
                rows.push(TreeRow::Run { sub: si, ses: None, run: ri });
            }
            // Sessions
            for (sei, ses) in sub.sessions.iter().enumerate() {
                rows.push(TreeRow::Session(si, sei));
                if self.collapsed.contains(&format!("sub-{}/ses-{}", sub.name, ses.name)) {
                    continue;
                }
                for ri in 0..ses.runs.len() {
                    rows.push(TreeRow::Run { sub: si, ses: Some(sei), run: ri });
                }
            }
        }
        rows
    }

    /// Total number of focusable items: pattern + tree rows + num_echoes.
    fn focusable_count(&self) -> usize {
        // pattern(1) + tree rows + num_echoes(1)
        1 + self.visible_rows().len() + 1
    }

    /// Move focus down.
    pub fn focus_next(&mut self) {
        let max = self.focusable_count().saturating_sub(1);
        match self.focus {
            FilterFocus::Pattern => {
                if !self.visible_rows().is_empty() {
                    self.focus = FilterFocus::TreeNode(0);
                } else {
                    self.focus = FilterFocus::NumEchoes;
                }
            }
            FilterFocus::TreeNode(i) => {
                let rows = self.visible_rows().len();
                if i + 1 < rows {
                    self.focus = FilterFocus::TreeNode(i + 1);
                } else {
                    self.focus = FilterFocus::NumEchoes;
                }
            }
            FilterFocus::NumEchoes => {} // already at bottom
        }
        let _ = max; // used for bounds
    }

    /// Move focus up.
    pub fn focus_prev(&mut self) {
        match self.focus {
            FilterFocus::Pattern => {} // already at top
            FilterFocus::TreeNode(0) => self.focus = FilterFocus::Pattern,
            FilterFocus::TreeNode(i) => self.focus = FilterFocus::TreeNode(i - 1),
            FilterFocus::NumEchoes => {
                let rows = self.visible_rows().len();
                if rows > 0 {
                    self.focus = FilterFocus::TreeNode(rows - 1);
                } else {
                    self.focus = FilterFocus::Pattern;
                }
            }
        }
    }

    /// Scan BIDS directory if it changed since last scan.
    pub fn maybe_rescan(&mut self, bids_dir: &str) {
        let dir = bids_dir.trim().to_string();
        if dir.is_empty() {
            return;
        }
        if self.scanned_bids_dir.as_deref() == Some(&dir) {
            return;
        }
        match discovery::scan_bids_tree(Path::new(&dir)) {
            Ok(tree) => {
                self.tree = Some(tree);
                self.focus = FilterFocus::Pattern;
                self.scroll_offset = 0;
            }
            Err(_) => {
                self.tree = None;
            }
        }
        self.scanned_bids_dir = Some(dir);
    }

    /// Apply glob pattern: check runs whose display matches, uncheck others.
    pub fn apply_pattern(&mut self) {
        let Some(ref mut tree) = self.tree else { return };
        let pat = self.pattern.trim();
        if pat.is_empty() {
            // Empty pattern: select all
            tree.set_all(true);
            return;
        }
        // Convert glob to regex: escape dots, * -> .*, ? -> .
        let regex_str = format!(
            "^(?i){}$",
            pat.replace('.', r"\.")
               .replace('*', ".*")
               .replace('?', ".")
        );
        let re = match regex::Regex::new(&regex_str) {
            Ok(r) => r,
            Err(_) => return,
        };
        tree.for_each_run_mut(|run| {
            run.selected = re.is_match(&run.key_string) || re.is_match(&run.display);
        });
    }

    /// Toggle the focused tree node (run leaf or subject/session toggle).
    pub fn toggle_focused(&mut self) {
        let rows = self.visible_rows();
        let FilterFocus::TreeNode(idx) = self.focus else { return };
        let Some(row) = rows.get(idx) else { return };
        let Some(ref mut tree) = self.tree else { return };

        match row {
            TreeRow::Subject(si) => {
                let sub = &tree.subjects[*si];
                let new_val = sub.selected_runs() < sub.total_runs();
                tree.subjects[*si].set_all(new_val);
            }
            TreeRow::Session(si, sei) => {
                let ses = &tree.subjects[*si].sessions[*sei];
                let new_val = ses.runs.iter().any(|r| !r.selected);
                tree.subjects[*si].sessions[*sei].set_all(new_val);
            }
            TreeRow::Run { sub, ses, run } => {
                match ses {
                    Some(sei) => {
                        let r = &mut tree.subjects[*sub].sessions[*sei].runs[*run];
                        r.selected = !r.selected;
                    }
                    None => {
                        let r = &mut tree.subjects[*sub].runs[*run];
                        r.selected = !r.selected;
                    }
                }
            }
        }
    }

    /// Toggle collapse on the focused subject or session.
    pub fn toggle_collapse(&mut self) {
        let rows = self.visible_rows();
        let FilterFocus::TreeNode(idx) = self.focus else { return };
        let Some(row) = rows.get(idx) else { return };
        let Some(ref tree) = self.tree else { return };

        let key = match row {
            TreeRow::Subject(si) => format!("sub-{}", tree.subjects[*si].name),
            TreeRow::Session(si, sei) => format!("sub-{}/ses-{}", tree.subjects[*si].name, tree.subjects[*si].sessions[*sei].name),
            _ => return,
        };
        if self.collapsed.contains(&key) {
            self.collapsed.remove(&key);
        } else {
            self.collapsed.insert(key);
        }
    }

    /// Collect selected runs as filter values for RunArgs.
    /// Returns (subjects, sessions, acquisitions, runs) as Option<Vec<String>>.
    /// None means "all" (no filter).
    #[allow(clippy::type_complexity)]
    pub fn selected_filters(&self) -> (Option<Vec<String>>, Option<Vec<String>>, Option<Vec<String>>, Option<Vec<String>>) {
        let Some(ref tree) = self.tree else {
            return (None, None, None, None);
        };
        if tree.total_runs() == 0 || tree.selected_runs() == tree.total_runs() {
            return (None, None, None, None);
        }

        let mut subjects = HashSet::new();
        let mut sessions = HashSet::new();
        let mut acquisitions = HashSet::new();
        let mut runs = HashSet::new();

        for sub in &tree.subjects {
            for run in &sub.runs {
                if run.selected {
                    subjects.insert(format!("sub-{}", sub.name));
                    Self::extract_entities_from_key(&run.key_string, &mut sessions, &mut acquisitions, &mut runs);
                }
            }
            for ses in &sub.sessions {
                for run in &ses.runs {
                    if run.selected {
                        subjects.insert(format!("sub-{}", sub.name));
                        sessions.insert(format!("ses-{}", ses.name));
                        Self::extract_entities_from_key(&run.key_string, &mut sessions, &mut acquisitions, &mut runs);
                    }
                }
            }
        }

        // Only filter on entities that actually narrow the selection
        let to_opt = |set: HashSet<String>| -> Option<Vec<String>> {
            if set.is_empty() { None } else {
                let mut v: Vec<_> = set.into_iter().collect();
                v.sort();
                Some(v)
            }
        };

        (to_opt(subjects), None, None, None)
        // We pass subjects only — the pipeline's DiscoveryFilter will handle the rest.
        // More granular filtering would require per-run selection which the current
        // DiscoveryFilter doesn't support. Subject-level is the primary use case.
    }

    fn extract_entities_from_key(
        _key: &str,
        _sessions: &mut HashSet<String>,
        _acquisitions: &mut HashSet<String>,
        _runs: &mut HashSet<String>,
    ) {
        // Key format: sub-XX[_ses-YY][_acq-ZZ][_run-WW]_SUFFIX
        // Entity extraction is handled by the pipeline's DiscoveryFilter
    }
}

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
    pub filter_state: FilterTreeState,
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
            // Tab 1: Filters (custom rendering — see FilterTreeState)
            vec![],
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
            filter_state: FilterTreeState::default(),
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
        // Route tab 1 (Filters) to its own handler
        if self.active_tab == 1 {
            self.handle_filter_key(key);
            return;
        }

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

    // ─── Filter tab key handling ───

    fn handle_filter_key(&mut self, key: KeyEvent) {
        // Handle editing mode (pattern or num_echoes text input)
        if self.filter_state.pattern_editing {
            self.handle_filter_pattern_key(key);
            return;
        }
        if self.filter_state.num_echoes_editing {
            self.handle_filter_num_echoes_key(key);
            return;
        }

        // Navigation mode
        match key.code {
            KeyCode::Char('q') | KeyCode::Esc => self.should_quit = true,

            // Tab switching (same as other tabs)
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

            // Navigation within filter tree
            KeyCode::Up | KeyCode::Char('k') => self.filter_state.focus_prev(),
            KeyCode::Down | KeyCode::Char('j') => self.filter_state.focus_next(),

            // Collapse/expand
            KeyCode::Left => self.filter_state.toggle_collapse(),
            KeyCode::Right => self.filter_state.toggle_collapse(),

            // Toggle / interact
            KeyCode::Char(' ') => self.filter_state.toggle_focused(),
            KeyCode::Enter => {
                match self.filter_state.focus {
                    FilterFocus::Pattern => {
                        self.filter_state.pattern_editing = true;
                        self.filter_state.pattern_cursor = self.filter_state.pattern.len();
                    }
                    FilterFocus::TreeNode(_) => self.filter_state.toggle_focused(),
                    FilterFocus::NumEchoes => {
                        self.filter_state.num_echoes_editing = true;
                        self.filter_state.num_echoes_cursor = self.filter_state.num_echoes.len();
                    }
                }
            }

            // Select all / none
            KeyCode::Char('a') => {
                if let Some(ref mut tree) = self.filter_state.tree {
                    tree.set_all(true);
                }
            }
            KeyCode::Char('n') => {
                if let Some(ref mut tree) = self.filter_state.tree {
                    tree.set_all(false);
                }
            }

            KeyCode::F(5) => self.should_run = true,

            _ => {}
        }
    }

    fn handle_filter_pattern_key(&mut self, key: KeyEvent) {
        let fs = &mut self.filter_state;
        match key.code {
            KeyCode::Esc => {
                fs.pattern_editing = false;
            }
            KeyCode::Enter => {
                fs.pattern_editing = false;
                fs.apply_pattern();
            }
            KeyCode::Char(c) => {
                fs.pattern.insert(fs.pattern_cursor, c);
                fs.pattern_cursor += 1;
            }
            KeyCode::Backspace if fs.pattern_cursor > 0 => {
                fs.pattern_cursor -= 1;
                fs.pattern.remove(fs.pattern_cursor);
            }
            KeyCode::Delete
                if fs.pattern_cursor < fs.pattern.len() => {
                    fs.pattern.remove(fs.pattern_cursor);
                }
            KeyCode::Left => fs.pattern_cursor = fs.pattern_cursor.saturating_sub(1),
            KeyCode::Right
                if fs.pattern_cursor < fs.pattern.len() => {
                    fs.pattern_cursor += 1;
                }
            KeyCode::Home => fs.pattern_cursor = 0,
            KeyCode::End => fs.pattern_cursor = fs.pattern.len(),
            _ => {}
        }
    }

    fn handle_filter_num_echoes_key(&mut self, key: KeyEvent) {
        let fs = &mut self.filter_state;
        match key.code {
            KeyCode::Esc | KeyCode::Enter => {
                fs.num_echoes_editing = false;
            }
            KeyCode::Char(c) => {
                fs.num_echoes.insert(fs.num_echoes_cursor, c);
                fs.num_echoes_cursor += 1;
            }
            KeyCode::Backspace if fs.num_echoes_cursor > 0 => {
                fs.num_echoes_cursor -= 1;
                fs.num_echoes.remove(fs.num_echoes_cursor);
            }
            KeyCode::Delete
                if fs.num_echoes_cursor < fs.num_echoes.len() => {
                    fs.num_echoes.remove(fs.num_echoes_cursor);
                }
            KeyCode::Left => fs.num_echoes_cursor = fs.num_echoes_cursor.saturating_sub(1),
            KeyCode::Right
                if fs.num_echoes_cursor < fs.num_echoes.len() => {
                    fs.num_echoes_cursor += 1;
                }
            KeyCode::Home => fs.num_echoes_cursor = 0,
            KeyCode::End => fs.num_echoes_cursor = fs.num_echoes.len(),
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
        assert_eq!(app.field_count(), 0); // Tab 1: Filters (custom rendering)
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
    fn test_filter_tab_routes_to_filter_handler() {
        let mut app = App::new();
        app.active_tab = 1;
        // Should not crash — filter handler takes over
        app.handle_key(key(KeyCode::Down));
        app.handle_key(key(KeyCode::Up));
        app.handle_key(key(KeyCode::Char('a')));
        app.handle_key(key(KeyCode::Char('n')));
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

    // --- Filter tree tests ---

    #[test]
    fn test_filter_state_default() {
        let fs = super::FilterTreeState::default();
        assert!(fs.tree.is_none());
        assert!(fs.pattern.is_empty());
        assert_eq!(fs.focus, super::FilterFocus::Pattern);
    }

    #[test]
    fn test_filter_scan_with_bids() {
        let dir = tempfile::tempdir().unwrap();
        crate::testutils::create_multi_echo_bids(dir.path());
        let mut fs = super::FilterTreeState::default();
        fs.maybe_rescan(dir.path().to_str().unwrap());
        assert!(fs.tree.is_some());
        let tree = fs.tree.as_ref().unwrap();
        assert_eq!(tree.subjects.len(), 1);
        assert_eq!(tree.subjects[0].name, "1");
    }

    #[test]
    fn test_filter_scan_empty_dir() {
        let dir = tempfile::tempdir().unwrap();
        let mut fs = super::FilterTreeState::default();
        fs.maybe_rescan(dir.path().to_str().unwrap());
        // Tree may be Some with empty subjects, or None
        if let Some(ref tree) = fs.tree {
            assert!(tree.subjects.is_empty());
        }
    }

    #[test]
    fn test_filter_scan_caches() {
        let dir = tempfile::tempdir().unwrap();
        crate::testutils::create_single_echo_bids(dir.path());
        let mut fs = super::FilterTreeState::default();
        let path = dir.path().to_str().unwrap();
        fs.maybe_rescan(path);
        assert!(fs.tree.is_some());
        // Second call should not rescan
        fs.maybe_rescan(path);
        assert_eq!(fs.scanned_bids_dir.as_deref(), Some(path));
    }

    #[test]
    fn test_filter_navigation() {
        let dir = tempfile::tempdir().unwrap();
        crate::testutils::create_single_echo_bids(dir.path());
        let mut fs = super::FilterTreeState::default();
        fs.maybe_rescan(dir.path().to_str().unwrap());

        assert_eq!(fs.focus, super::FilterFocus::Pattern);
        fs.focus_next(); // -> tree node 0 (subject)
        assert!(matches!(fs.focus, super::FilterFocus::TreeNode(0)));
        fs.focus_next(); // -> tree node 1 (run)
        fs.focus_next(); // -> NumEchoes
        assert_eq!(fs.focus, super::FilterFocus::NumEchoes);
        fs.focus_next(); // stays at NumEchoes
        assert_eq!(fs.focus, super::FilterFocus::NumEchoes);
        fs.focus_prev(); // back up
        assert!(matches!(fs.focus, super::FilterFocus::TreeNode(_)));
    }

    #[test]
    fn test_filter_toggle_run() {
        let dir = tempfile::tempdir().unwrap();
        crate::testutils::create_single_echo_bids(dir.path());
        let mut fs = super::FilterTreeState::default();
        fs.maybe_rescan(dir.path().to_str().unwrap());

        // Navigate to the run leaf
        fs.focus_next(); // subject
        fs.focus_next(); // run leaf
        let tree = fs.tree.as_ref().unwrap();
        assert!(tree.subjects[0].runs[0].selected);
        fs.toggle_focused();
        let tree = fs.tree.as_ref().unwrap();
        assert!(!tree.subjects[0].runs[0].selected);
    }

    #[test]
    fn test_filter_select_all_none() {
        let dir = tempfile::tempdir().unwrap();
        crate::testutils::create_multi_echo_bids(dir.path());
        let mut fs = super::FilterTreeState::default();
        fs.maybe_rescan(dir.path().to_str().unwrap());

        let tree = fs.tree.as_mut().unwrap();
        tree.set_all(false);
        assert_eq!(tree.selected_runs(), 0);
        tree.set_all(true);
        assert_eq!(tree.selected_runs(), tree.total_runs());
    }

    #[test]
    fn test_filter_pattern_editing() {
        let mut app = App::new();
        app.active_tab = 1;
        let dir = tempfile::tempdir().unwrap();
        crate::testutils::create_single_echo_bids(dir.path());
        app.form.bids_dir = dir.path().to_str().unwrap().to_string();
        app.filter_state.maybe_rescan(&app.form.bids_dir.clone());

        // Enter pattern editing
        app.filter_state.focus = super::FilterFocus::Pattern;
        app.handle_key(key(KeyCode::Enter));
        assert!(app.filter_state.pattern_editing);

        // Type something
        app.handle_key(key(KeyCode::Char('*')));
        assert_eq!(app.filter_state.pattern, "*");

        // Esc cancels
        app.handle_key(key(KeyCode::Esc));
        assert!(!app.filter_state.pattern_editing);
    }

    #[test]
    fn test_filter_num_echoes_editing() {
        let mut app = App::new();
        app.active_tab = 1;
        app.filter_state.focus = super::FilterFocus::NumEchoes;
        app.handle_key(key(KeyCode::Enter));
        assert!(app.filter_state.num_echoes_editing);
        app.handle_key(key(KeyCode::Char('3')));
        assert_eq!(app.filter_state.num_echoes, "3");
        app.handle_key(key(KeyCode::Enter));
        assert!(!app.filter_state.num_echoes_editing);
    }

    #[test]
    fn test_filter_collapse() {
        let dir = tempfile::tempdir().unwrap();
        crate::testutils::create_single_echo_bids(dir.path());
        let mut fs = super::FilterTreeState::default();
        fs.maybe_rescan(dir.path().to_str().unwrap());

        let rows_before = fs.visible_rows().len();
        fs.focus = super::FilterFocus::TreeNode(0); // subject node
        fs.toggle_collapse();
        let rows_after = fs.visible_rows().len();
        assert!(rows_after < rows_before, "Collapsing should hide children");

        // Expand again
        fs.toggle_collapse();
        assert_eq!(fs.visible_rows().len(), rows_before);
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
