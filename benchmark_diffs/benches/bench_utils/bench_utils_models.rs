use std::{
    fmt::{Debug, Display},
    fs,
    path::{Path, PathBuf},
};

use serde::Serialize;

pub enum HeuristicType {
    Lazy,
    Greedy,
}

#[derive(Clone, Copy)]
pub enum Heuristic {
    Greedy200,
    Simple,
    LazySimple,
    LazyGreedy200,
    LazyGreedy1000,
}

impl Display for Heuristic {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Heuristic::Greedy200 => write!(f, "Gumtree_Greedy_200"),
            Heuristic::Simple => write!(f, "Gumtree_Simple"),
            Heuristic::LazyGreedy200 => write!(f, "Lazy_Gumtree_Greedy_200"),
            Heuristic::LazyGreedy1000 => write!(f, "Lazy_Gumtree_Greedy_1000"),
            Heuristic::LazySimple => write!(f, "Lazy_Gumtree_Simple"),
        }
    }
}

impl Heuristic {
    pub fn get_heuristic_type(&self) -> HeuristicType {
        match self {
            Heuristic::Greedy200 => HeuristicType::Greedy,
            Heuristic::Simple => HeuristicType::Greedy,
            Heuristic::LazyGreedy200 => HeuristicType::Lazy,
            Heuristic::LazyGreedy1000 => HeuristicType::Lazy,
            Heuristic::LazySimple => HeuristicType::Lazy,
        }
    }
}

/// Four subsets of the total dataset. The optional string is a specific project in the subset, None if we want all.
#[derive(Debug, Clone, Serialize)]
pub enum DataSet {
    GhJava(Option<String>),
    GhPython(Option<String>),
    Defects4J(Option<String>),
    BugsInPy(Option<String>),
}

impl Display for DataSet {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let (dataset_name, project_opt) = self.parts();
        let project_name = project_opt.as_deref().unwrap_or_default();
        write!(f, "{}/{}", dataset_name, project_name)
    }
}

impl DataSet {
    /// Returns a tuple of (dataset_name, project_name_option)
    fn parts(&self) -> (&'static str, &Option<String>) {
        match self {
            DataSet::GhJava(project) => ("gh-java", project),
            DataSet::GhPython(project) => ("gh-python", project),
            DataSet::Defects4J(project) => ("defects4j", project),
            DataSet::BugsInPy(project) => ("bugsinpy", project),
        }
    }

    fn get_base_path() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .parent()
            .unwrap()
            .join("datasets")
    }

    fn get_directory_names(path: &PathBuf) -> Vec<String> {
        assert!(path.exists(), "Path does not exist: {:?}", path);
        assert!(path.is_dir(), "Path is not a directory: {:?}", path);
        let mut dir_names = Vec::new();

        for entry_result in fs::read_dir(path).expect("Could not read dir") {
            let entry = entry_result.expect("Error when reading file entry");
            let path = entry.path();

            if path.is_dir() {
                if let Some(os_str_name) = path.file_name() {
                    if let Some(name_str) = os_str_name.to_str() {
                        dir_names.push(name_str.to_string());
                    } else {
                        eprintln!("Skipping non-UTF-8 directory name: {:?}", os_str_name);
                    }
                }
            }
        }

        dir_names
    }

    pub fn get_all_projects_of_dataset(&self) -> Vec<String> {
        let base_path = DataSet::get_base_path();
        let (dataset_name, _) = self.parts();
        let path = base_path.join(dataset_name).join("before");
        DataSet::get_directory_names(&path)
    }

    pub fn get_path_dataset_project(&self) -> (PathBuf, PathBuf) {
        let dataset_root = DataSet::get_base_path();

        let (dataset_name, opt_project_name) = self.parts();
        let project_name = opt_project_name.as_deref().unwrap_or_default();

        let full_path_before = dataset_root
            .join(dataset_name)
            .join("before")
            .join(project_name);
        let full_path_after = dataset_root
            .join(dataset_name)
            .join("after")
            .join(project_name);

        assert!(
            dataset_root.exists(),
            "Path to dataset did not exist, path was: {:?}",
            dataset_root.display()
        );
        assert!(
            full_path_before.exists(),
            "Path to dataset before subset did not exist, path was: {:?}",
            full_path_before.display()
        );
        assert!(
            full_path_after.exists(),
            "Path to dataset before subset did not exist, path was: {:?}",
            full_path_after.display()
        );
        (full_path_before, full_path_after)
    }

    pub fn generate_bench_group_name(&self) -> String {
        let (dataset_name, opt_project_name) = self.parts();
        let project_name = opt_project_name.as_deref().unwrap_or_default();
        format!("{}_{}", dataset_name, project_name)
    }
}
