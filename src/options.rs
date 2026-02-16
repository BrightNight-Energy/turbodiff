#[derive(Clone, Debug)]
pub struct DeepDiffOptions {
    pub(crate) ignore_order: bool,
    pub(crate) ignore_numeric_type_changes: bool,
    pub(crate) ignore_string_type_changes: bool,
    pub(crate) significant_digits: Option<u32>,
    pub(crate) math_epsilon: Option<f64>,
    pub(crate) atol: Option<f64>,
    pub(crate) rtol: Option<f64>,
    pub(crate) include_paths: Vec<String>,
    pub(crate) exclude_paths: Vec<String>,
    pub(crate) verbose_level: u8,
    pub(crate) ignore_type_in_groups: Vec<Vec<ValueType>>,
}

impl Default for DeepDiffOptions {
    fn default() -> Self {
        Self {
            ignore_order: false,
            ignore_numeric_type_changes: false,
            ignore_string_type_changes: false,
            significant_digits: None,
            math_epsilon: None,
            atol: None,
            rtol: None,
            include_paths: Vec::new(),
            exclude_paths: Vec::new(),
            verbose_level: 1,
            ignore_type_in_groups: Vec::new(),
        }
    }
}

impl DeepDiffOptions {
    pub fn ignore_order(mut self, value: bool) -> Self {
        self.ignore_order = value;
        self
    }

    pub fn ignore_numeric_type_changes(mut self, value: bool) -> Self {
        self.ignore_numeric_type_changes = value;
        self
    }

    pub fn ignore_string_type_changes(mut self, value: bool) -> Self {
        self.ignore_string_type_changes = value;
        self
    }

    pub fn significant_digits(mut self, value: Option<u32>) -> Self {
        self.significant_digits = value;
        self
    }

    pub fn math_epsilon(mut self, value: Option<f64>) -> Self {
        self.math_epsilon = value;
        self
    }

    pub fn atol(mut self, value: Option<f64>) -> Self {
        self.atol = value;
        self
    }

    pub fn rtol(mut self, value: Option<f64>) -> Self {
        self.rtol = value;
        self
    }

    pub fn include_paths(mut self, paths: Vec<String>) -> Self {
        self.include_paths = paths;
        self
    }

    pub fn exclude_paths(mut self, paths: Vec<String>) -> Self {
        self.exclude_paths = paths;
        self
    }

    pub fn verbose_level(mut self, value: u8) -> Self {
        self.verbose_level = value;
        self
    }

    pub fn ignore_type_in_groups(mut self, groups: Vec<Vec<ValueType>>) -> Self {
        self.ignore_type_in_groups = groups;
        self
    }
}

#[derive(Clone, Debug)]
pub struct PrettyOptions {
    pub compact: bool,
    pub max_depth: usize,
    pub context: usize,
    pub no_color: bool,
    pub path_header: bool,
}

impl Default for PrettyOptions {
    fn default() -> Self {
        Self {
            compact: false,
            max_depth: 5,
            context: 0,
            no_color: false,
            path_header: false,
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum ValueType {
    Number,
    String,
    Bool,
    Null,
    Array,
    Object,
}
