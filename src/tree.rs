use std::fs;
use std::io;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileNode {
    pub path: PathBuf,
    pub name: String,
    pub is_dir: bool,
    pub is_expanded: bool,
    pub depth: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileTree {
    root_path: PathBuf,
    entries: Vec<FileNode>,
    selected_index: usize,
}

fn scan_directory(dir_path: &Path, depth: usize) -> io::Result<Vec<FileNode>> {
    let mut children = Vec::new();
    if dir_path.is_dir() {
        for entry in fs::read_dir(dir_path)? {
            let entry = entry?;
            let path = entry.path();
            let name = entry.file_name().to_string_lossy().into_owned();
            let file_type = entry.file_type()?;
            let is_dir = file_type.is_dir();

            children.push(FileNode {
                path,
                name,
                is_dir,
                is_expanded: false,
                depth,
            });
        }

        children.sort_by(|a, b| match (a.is_dir, b.is_dir) {
            (true, false) => std::cmp::Ordering::Less,
            (false, true) => std::cmp::Ordering::Greater,
            _ => a.name.cmp(&b.name),
        });
    }
    Ok(children)
}

impl FileTree {
    pub fn new(root_path: impl AsRef<Path>) -> io::Result<Self> {
        let root_path = root_path.as_ref().to_path_buf();
        let entries = scan_directory(&root_path, 0)?;

        Ok(Self {
            root_path,
            entries,
            selected_index: 0,
        })
    }

    pub fn root_path(&self) -> &Path {
        &self.root_path
    }

    pub fn entries(&self) -> &[FileNode] {
        &self.entries
    }

    pub fn selected_index(&self) -> usize {
        self.selected_index
    }

    pub fn selected_entry(&self) -> Option<&FileNode> {
        self.entries.get(self.selected_index)
    }

    pub fn move_up(&mut self) {
        if self.selected_index > 0 {
            self.selected_index -= 1;
        }
    }

    pub fn move_down(&mut self) {
        if !self.entries.is_empty() && self.selected_index + 1 < self.entries.len() {
            self.selected_index += 1;
        }
    }

    pub fn expand_or_select_child(&mut self) {
        if self.entries.is_empty() || self.selected_index >= self.entries.len() {
            return;
        }

        let node = &self.entries[self.selected_index];
        if !node.is_dir {
            return;
        }

        if node.is_expanded {
            if self.selected_index + 1 < self.entries.len()
                && self.entries[self.selected_index + 1].depth == node.depth + 1
            {
                self.selected_index += 1;
            }
        } else {
            let target_path = node.path.clone();
            let target_depth = node.depth + 1;
            self.entries[self.selected_index].is_expanded = true;

            if let Ok(children) = scan_directory(&target_path, target_depth) {
                let insert_pos = self.selected_index + 1;
                for (i, child) in children.into_iter().enumerate() {
                    self.entries.insert(insert_pos + i, child);
                }
            }
        }
    }

    pub fn collapse_or_select_parent(&mut self) {
        if self.entries.is_empty() || self.selected_index >= self.entries.len() {
            return;
        }

        let node = &self.entries[self.selected_index];

        if node.is_dir && node.is_expanded {
            let parent_depth = node.depth;
            self.entries[self.selected_index].is_expanded = false;

            let mut remove_count = 0;
            while self.selected_index + 1 + remove_count < self.entries.len() {
                if self.entries[self.selected_index + 1 + remove_count].depth > parent_depth {
                    remove_count += 1;
                } else {
                    break;
                }
            }

            if remove_count > 0 {
                self.entries.drain((self.selected_index + 1)..(self.selected_index + 1 + remove_count));
            }
        } else if node.depth > 0 {
            let target_depth = node.depth - 1;
            for i in (0..self.selected_index).rev() {
                if self.entries[i].depth == target_depth {
                    self.selected_index = i;
                    break;
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::{self, File};

    #[test]
    fn test_file_tree_scanning_and_sorting() {
        let temp_dir = std::env::temp_dir().join(format!("splash_test_tree_{}", std::process::id()));
        let _ = fs::remove_dir_all(&temp_dir);
        fs::create_dir_all(&temp_dir).unwrap();

        // Create files and folders (including dotfiles)
        fs::create_dir_all(temp_dir.join("src")).unwrap();
        fs::create_dir_all(temp_dir.join(".git")).unwrap();
        File::create(temp_dir.join("b_file.txt")).unwrap();
        File::create(temp_dir.join("a_file.txt")).unwrap();
        File::create(temp_dir.join(".gitignore")).unwrap();

        let tree = FileTree::new(&temp_dir).unwrap();

        let names: Vec<&str> = tree.entries().iter().map(|n| n.name.as_str()).collect();

        let _ = fs::remove_dir_all(&temp_dir);

        // Folders first (alphabetically: ".git", "src"), then files (alphabetically: ".gitignore", "a_file.txt", "b_file.txt")
        assert_eq!(names, vec![".git", "src", ".gitignore", "a_file.txt", "b_file.txt"]);
        assert_eq!(tree.selected_index(), 0);
    }

    #[test]
    fn test_file_tree_cursor_navigation() {
        let temp_dir = std::env::temp_dir().join(format!("splash_test_tree_nav_{}", std::process::id()));
        let _ = fs::remove_dir_all(&temp_dir);
        fs::create_dir_all(&temp_dir).unwrap();
        File::create(temp_dir.join("a.txt")).unwrap();
        File::create(temp_dir.join("b.txt")).unwrap();
        File::create(temp_dir.join("c.txt")).unwrap();

        let mut tree = FileTree::new(&temp_dir).unwrap();
        assert_eq!(tree.entries().len(), 3);
        assert_eq!(tree.selected_index(), 0);

        // move down
        tree.move_down();
        assert_eq!(tree.selected_index(), 1);
        tree.move_down();
        assert_eq!(tree.selected_index(), 2);

        // move down past end (should clamp at 2)
        tree.move_down();
        assert_eq!(tree.selected_index(), 2);

        // move up
        tree.move_up();
        assert_eq!(tree.selected_index(), 1);
        tree.move_up();
        assert_eq!(tree.selected_index(), 0);

        // move up past top (should clamp at 0)
        tree.move_up();
        assert_eq!(tree.selected_index(), 0);

        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_file_tree_empty_directory() {
        let temp_dir = std::env::temp_dir().join(format!("splash_test_tree_empty_{}", std::process::id()));
        let _ = fs::remove_dir_all(&temp_dir);
        fs::create_dir_all(&temp_dir).unwrap();

        let mut tree = FileTree::new(&temp_dir).unwrap();
        assert_eq!(tree.entries().len(), 0);
        assert_eq!(tree.selected_index(), 0);
        assert_eq!(tree.selected_entry(), None);

        // Navigation on empty tree should not panic
        tree.move_down();
        assert_eq!(tree.selected_index(), 0);
        tree.move_up();
        assert_eq!(tree.selected_index(), 0);

        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_expand_or_select_child() {
        let temp_dir = std::env::temp_dir().join(format!("splash_test_tree_expand_{}", std::process::id()));
        let _ = fs::remove_dir_all(&temp_dir);
        fs::create_dir_all(temp_dir.join("sub")).unwrap();
        File::create(temp_dir.join("root_file.txt")).unwrap();
        File::create(temp_dir.join("sub/nested_file.txt")).unwrap();
        fs::create_dir_all(temp_dir.join("sub/nested_dir")).unwrap();

        let mut tree = FileTree::new(&temp_dir).unwrap();
        assert_eq!(tree.entries().len(), 2);
        assert_eq!(tree.entries()[0].name, "sub");
        assert_eq!(tree.entries()[0].is_expanded, false);

        // First call on collapsed folder "sub": expands and loads children, cursor stays at index 0
        tree.expand_or_select_child();

        assert_eq!(tree.entries()[0].is_expanded, true);
        assert_eq!(tree.entries().len(), 4);
        assert_eq!(tree.entries()[1].name, "nested_dir");
        assert_eq!(tree.entries()[1].depth, 1);
        assert_eq!(tree.entries()[2].name, "nested_file.txt");
        assert_eq!(tree.entries()[2].depth, 1);
        assert_eq!(tree.entries()[3].name, "root_file.txt");
        assert_eq!(tree.entries()[3].depth, 0);
        assert_eq!(tree.selected_index(), 0);

        // Second call on already expanded folder "sub": moves selection to first child (index 1)
        tree.expand_or_select_child();
        assert_eq!(tree.selected_index(), 1);

        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_collapse_expanded_folder() {
        let temp_dir = std::env::temp_dir().join(format!("splash_test_tree_collapse_{}", std::process::id()));
        let _ = fs::remove_dir_all(&temp_dir);
        fs::create_dir_all(temp_dir.join("sub/nested_dir")).unwrap();
        File::create(temp_dir.join("root_file.txt")).unwrap();
        File::create(temp_dir.join("sub/nested_file.txt")).unwrap();
        File::create(temp_dir.join("sub/nested_dir/deep_file.txt")).unwrap();

        let mut tree = FileTree::new(&temp_dir).unwrap();
        // Expand "sub"
        tree.expand_or_select_child();
        // Move to "nested_dir" (index 1) and expand it
        tree.move_down();
        tree.expand_or_select_child();

        assert_eq!(tree.entries().len(), 5);

        // Move cursor back to "sub" (index 0)
        tree.move_up();
        assert_eq!(tree.selected_index(), 0);
        assert_eq!(tree.entries()[0].name, "sub");

        // Collapse "sub"
        tree.collapse_or_select_parent();

        assert_eq!(tree.entries()[0].is_expanded, false);
        assert_eq!(tree.entries().len(), 2);
        assert_eq!(tree.entries()[0].name, "sub");
        assert_eq!(tree.entries()[1].name, "root_file.txt");
        assert_eq!(tree.selected_index(), 0);

        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_select_parent_folder() {
        let temp_dir = std::env::temp_dir().join(format!("splash_test_tree_parent_{}", std::process::id()));
        let _ = fs::remove_dir_all(&temp_dir);
        fs::create_dir_all(temp_dir.join("sub/nested_dir")).unwrap();
        File::create(temp_dir.join("root_file.txt")).unwrap();
        File::create(temp_dir.join("sub/nested_file.txt")).unwrap();
        File::create(temp_dir.join("sub/nested_dir/deep_file.txt")).unwrap();

        let mut tree = FileTree::new(&temp_dir).unwrap();
        // Expand "sub" (index 0)
        tree.expand_or_select_child();
        // Move to "nested_dir" (index 1) and expand it
        tree.move_down();
        tree.expand_or_select_child();
        // Move to "deep_file.txt" (index 2)
        tree.move_down();

        assert_eq!(tree.entries()[tree.selected_index()].name, "deep_file.txt");
        assert_eq!(tree.entries()[tree.selected_index()].depth, 2);

        // Call collapse_or_select_parent on deep_file.txt -> should jump to parent "nested_dir" (index 1)
        tree.collapse_or_select_parent();
        assert_eq!(tree.selected_index(), 1);
        assert_eq!(tree.entries()[tree.selected_index()].name, "nested_dir");

        // Collapse "nested_dir" (index 1)
        tree.collapse_or_select_parent();
        assert_eq!(tree.entries()[1].name, "nested_dir");
        assert_eq!(tree.entries()[1].is_expanded, false);

        // Call collapse_or_select_parent on collapsed "nested_dir" (depth 1) -> should jump to parent "sub" (index 0)
        tree.collapse_or_select_parent();
        assert_eq!(tree.selected_index(), 0);
        assert_eq!(tree.entries()[tree.selected_index()].name, "sub");

        let _ = fs::remove_dir_all(&temp_dir);
    }
}




