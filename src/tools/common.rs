use crate::graph::DependencyType;
use std::collections::HashMap;

/// Deduplicates a list of modules by removing children when their parent is present,
/// and tracks how many original modules each deduplicated entry represents.
pub fn filter_hierarchical(
    mut modules: Vec<(String, DependencyType)>,
) -> Vec<(String, DependencyType, usize)> {
    // First, deduplicate exact module names, keeping first dependency type
    let mut seen_modules = HashMap::new();
    modules.retain(|(module_path, dep_type)| {
        if seen_modules.contains_key(module_path) {
            false
        } else {
            seen_modules.insert(module_path.clone(), dep_type.clone());
            true
        }
    });

    // Sort by path to ensure consistent processing
    modules.sort_by(|a, b| a.0.cmp(&b.0));

    let mut result = Vec::new();

    for (module_path, dep_type) in modules {
        // Check if any module already in result is a parent of this module
        let parent_index =
            result
                .iter()
                .position(|(existing_path, _, _): &(String, DependencyType, usize)| {
                    module_path.starts_with(&format!("{}.", existing_path))
                });

        if let Some(index) = parent_index {
            // This module is a child of an existing parent, increment the parent's count
            result[index].2 += 1;
        } else {
            // Count how many existing modules are children of this module
            let mut child_count = 1; // Count self
            let mut indices_to_remove = Vec::new();

            for (i, (existing_path, _, existing_count)) in result.iter().enumerate() {
                if existing_path.starts_with(&format!("{}.", module_path)) {
                    child_count += existing_count;
                    indices_to_remove.push(i);
                }
            }

            // Remove children in reverse order to maintain indices
            for &i in indices_to_remove.iter().rev() {
                result.remove(i);
            }

            result.push((module_path, dep_type, child_count));
        }
    }

    result
}

/// Common formatting functionality for hierarchical module display
pub mod formatters {
    use crate::graph::DependencyType;
    use std::collections::HashMap;

    // Constants for formatting
    const INDENT: &str = "  ";
    const DOT_SEPARATOR: &str = ".";

    /// Calculates prefix counts for hierarchical grouping
    pub fn calculate_prefix_counts(
        modules: &[(String, DependencyType, usize)],
    ) -> HashMap<String, usize> {
        let mut prefix_counts: HashMap<String, usize> = HashMap::new();
        for (module_path, _dep_type, count) in modules {
            let segments: Vec<&str> = module_path.split(DOT_SEPARATOR).collect();
            for i in 1..segments.len() {
                let prefix = segments[0..i].join(DOT_SEPARATOR);
                *prefix_counts.entry(prefix).or_insert(0) += count;
            }
        }
        prefix_counts
    }

    /// Formats a single segment with appropriate indentation and count
    pub fn format_segment(
        indent_level: usize,
        segment: &str,
        count: Option<usize>,
        is_root: bool,
    ) -> String {
        let indent = INDENT.repeat(indent_level + 1); // Fixed inefficient string building
        let prefix_char = if is_root { "" } else { DOT_SEPARATOR };

        match count {
            Some(c) if c > 1 => format!("{}{}{} ({})\n", indent, prefix_char, segment, c),
            _ => format!("{}{}{}\n", indent, prefix_char, segment),
        }
    }

    /// Finds the common prefix length between two module paths
    pub fn find_common_prefix_length(current: &[String], new: &[String]) -> usize {
        current
            .iter()
            .zip(new.iter())
            .take_while(|(a, b)| a == b)
            .count()
    }

    /// Main function for formatting modules with hierarchical grouping
    pub fn format_grouped_modules(modules: &[(String, DependencyType, usize)]) -> String {
        let mut output = String::new();
        let mut current_prefix: Vec<String> = Vec::new();
        let prefix_counts = calculate_prefix_counts(modules);

        for (module_path, _dep_type, count) in modules {
            let segments: Vec<String> = module_path
                .split(DOT_SEPARATOR)
                .map(|s| s.to_string())
                .collect();
            let common_len = find_common_prefix_length(&current_prefix, &segments);

            // Pre-compute path for efficiency
            let mut current_path = String::new();

            // Output new segments that differ from current prefix
            for (i, segment) in segments.iter().enumerate().skip(common_len) {
                let is_final_segment = i == segments.len() - 1;
                let is_root_segment = i == 0;

                // Build path incrementally instead of joining repeatedly
                if i == 0 {
                    current_path = segment.clone();
                } else {
                    current_path.push_str(DOT_SEPARATOR);
                    current_path.push_str(segment);
                }

                let segment_count = if is_final_segment {
                    // Final segment shows the module's count
                    if *count > 1 { Some(*count) } else { None }
                } else {
                    // Intermediate segment shows prefix count if > 1
                    prefix_counts.get(&current_path).copied().filter(|&c| c > 1)
                };

                output.push_str(&format_segment(i, segment, segment_count, is_root_segment));
            }

            current_prefix = segments;
        }

        output
    }
}
