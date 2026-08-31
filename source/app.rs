use std::{
    collections::{HashMap, HashSet},
    path::{Path, PathBuf},
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum InputMode {
    Normal,
    Search,
}

#[derive(Debug)]
pub(crate) struct ImageEntry {
    pub(crate) id: EntryId,
    pub(crate) revision: SourceRevision,
    pub(crate) path: PathBuf,
    pub(crate) label: String,
    search_key: String,
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub(crate) struct EntryId(pub(crate) u64);

#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub(crate) struct SourceRevision {
    pub(crate) size: u64,
    pub(crate) modified_nanoseconds: i128,
}

#[derive(Debug)]
pub(crate) struct App {
    pub(crate) root: PathBuf,
    pub(crate) entries: Vec<ImageEntry>,
    pub(crate) filtered: Vec<EntryId>,
    pub(crate) selected: usize,
    pub(crate) row_offset: usize,
    pub(crate) query: String,
    pub(crate) mode: InputMode,
    pub(crate) scan_done: bool,
    pub(crate) scan_errors: usize,
    pub(crate) thumbnail_errors: usize,
    marked: HashSet<EntryId>,
    mark_order: Vec<EntryId>,
    mark_anchor: Option<EntryId>,
    marked_only: bool,
    query_before_edit: String,
    entry_positions: HashMap<EntryId, usize>,
    path_ids: HashMap<PathBuf, EntryId>,
    next_entry_id: u64,
}

impl App {
    pub(crate) fn new(root: PathBuf) -> Self {
        Self {
            root,
            entries: Vec::new(),
            filtered: Vec::new(),
            selected: 0,
            row_offset: 0,
            query: String::new(),
            mode: InputMode::Normal,
            scan_done: false,
            scan_errors: 0,
            thumbnail_errors: 0,
            marked: HashSet::new(),
            mark_order: Vec::new(),
            mark_anchor: None,
            marked_only: false,
            query_before_edit: String::new(),
            entry_positions: HashMap::new(),
            path_ids: HashMap::new(),
            next_entry_id: 0,
        }
    }

    pub(crate) fn add_path(&mut self, path: PathBuf, revision: SourceRevision) -> EntryId {
        if let Some(id) = self.path_ids.get(&path).copied() {
            if let Some(entry) = self.entry_mut(id) {
                entry.revision = revision;
            }
            return id;
        }
        let id = EntryId(self.next_entry_id);
        self.next_entry_id = self.next_entry_id.saturating_add(1);
        let relative = path.strip_prefix(&self.root).unwrap_or(&path);
        let label = path
            .file_name()
            .unwrap_or_else(|| path.as_os_str())
            .to_string_lossy()
            .into_owned();
        let search_key = relative.to_string_lossy().to_lowercase();
        let matches = !self.marked_only
            && (self.query.is_empty() || search_key.contains(&self.query.to_lowercase()));

        self.entries.push(ImageEntry {
            id,
            revision,
            path,
            label,
            search_key,
        });
        if matches {
            self.filtered.push(id);
        }
        let position = self.entries.len() - 1;
        self.entry_positions.insert(id, position);
        self.path_ids
            .insert(self.entries[position].path.clone(), id);
        id
    }

    pub(crate) fn selected_entry(&self) -> Option<&ImageEntry> {
        self.filtered
            .get(self.selected)
            .and_then(|id| self.entry(*id))
    }

    pub(crate) fn entry(&self, id: EntryId) -> Option<&ImageEntry> {
        self.entry_positions
            .get(&id)
            .and_then(|position| self.entries.get(*position))
    }

    pub(crate) fn is_marked(&self, id: EntryId) -> bool {
        self.marked.contains(&id)
    }

    pub(crate) fn marked_count(&self) -> usize {
        self.marked.len()
    }

    pub(crate) const fn marked_only(&self) -> bool {
        self.marked_only
    }

    pub(crate) fn marked_paths(&self) -> Vec<PathBuf> {
        self.mark_order
            .iter()
            .filter(|id| self.marked.contains(id))
            .filter_map(|id| self.entry(*id).map(|entry| entry.path.clone()))
            .collect()
    }

    pub(crate) fn toggle_mark(&mut self) {
        let Some(id) = self.filtered.get(self.selected).copied() else {
            return;
        };
        self.toggle_mark_id(id);
    }

    pub(crate) fn toggle_mark_id(&mut self, id: EntryId) {
        if self.entry(id).is_none() {
            return;
        }
        if self.marked.remove(&id) {
            self.mark_order.retain(|candidate| *candidate != id);
        } else {
            self.marked.insert(id);
            self.mark_order.push(id);
        }
        self.mark_anchor = Some(id);
        if self.marked_only {
            self.rebuild_filter();
        }
    }

    pub(crate) fn mark_range(&mut self) {
        let Some(current) = self.filtered.get(self.selected).copied() else {
            return;
        };
        let anchor = self
            .mark_anchor
            .and_then(|id| self.filtered.iter().position(|candidate| *candidate == id))
            .unwrap_or(self.selected);
        let (start, end) = if anchor <= self.selected {
            (anchor, self.selected)
        } else {
            (self.selected, anchor)
        };
        for id in self.filtered[start..=end].iter().copied() {
            if self.marked.insert(id) {
                self.mark_order.push(id);
            }
        }
        self.mark_anchor.get_or_insert(current);
    }

    pub(crate) fn clear_marks(&mut self) {
        self.marked.clear();
        self.mark_order.clear();
        self.mark_anchor = None;
        if self.marked_only {
            self.rebuild_filter();
        }
    }

    pub(crate) fn toggle_marked_only(&mut self) {
        self.marked_only = !self.marked_only;
        self.rebuild_filter();
    }

    fn entry_mut(&mut self, id: EntryId) -> Option<&mut ImageEntry> {
        let position = *self.entry_positions.get(&id)?;
        self.entries.get_mut(position)
    }

    pub(crate) fn move_left(&mut self, columns: usize, rows: usize) {
        if self.selected > 0 {
            self.selected -= 1;
        }
        self.ensure_visible(columns, rows);
    }

    pub(crate) fn move_right(&mut self, columns: usize, rows: usize) {
        if self.selected + 1 < self.filtered.len() {
            self.selected += 1;
        }
        self.ensure_visible(columns, rows);
    }

    pub(crate) fn move_up(&mut self, columns: usize, rows: usize) {
        self.selected = self.selected.saturating_sub(columns.max(1));
        self.ensure_visible(columns, rows);
    }

    pub(crate) fn move_down(&mut self, columns: usize, rows: usize) {
        if !self.filtered.is_empty() {
            self.selected = (self.selected + columns.max(1)).min(self.filtered.len() - 1);
        }
        self.ensure_visible(columns, rows);
    }

    pub(crate) fn page_up(&mut self, columns: usize, rows: usize) {
        self.selected = self
            .selected
            .saturating_sub(columns.max(1).saturating_mul(rows.max(1)));
        self.ensure_visible(columns, rows);
    }

    pub(crate) fn page_down(&mut self, columns: usize, rows: usize) {
        if !self.filtered.is_empty() {
            self.selected = (self.selected + columns.max(1).saturating_mul(rows.max(1)))
                .min(self.filtered.len() - 1);
        }
        self.ensure_visible(columns, rows);
    }

    pub(crate) fn select_first(&mut self) {
        self.selected = 0;
        self.row_offset = 0;
    }

    pub(crate) fn select_last(&mut self, columns: usize, rows: usize) {
        self.selected = self.filtered.len().saturating_sub(1);
        self.ensure_visible(columns, rows);
    }

    pub(crate) fn start_search(&mut self) {
        self.query_before_edit.clone_from(&self.query);
        self.mode = InputMode::Search;
    }

    pub(crate) fn commit_search(&mut self) {
        self.mode = InputMode::Normal;
    }

    pub(crate) fn cancel_search(&mut self) {
        self.query = std::mem::take(&mut self.query_before_edit);
        self.mode = InputMode::Normal;
        self.rebuild_filter();
    }

    pub(crate) fn push_search(&mut self, character: char) {
        self.query.push(character);
        self.rebuild_filter();
    }

    pub(crate) fn pop_search(&mut self) {
        self.query.pop();
        self.rebuild_filter();
    }

    pub(crate) fn ensure_visible(&mut self, columns: usize, rows: usize) {
        let columns = columns.max(1);
        let rows = rows.max(1);
        let selected_row = self.selected / columns;
        if selected_row < self.row_offset {
            self.row_offset = selected_row;
        } else if selected_row >= self.row_offset + rows {
            self.row_offset = selected_row - rows + 1;
        }

        let total_rows = self.filtered.len().div_ceil(columns);
        self.row_offset = self.row_offset.min(total_rows.saturating_sub(rows));
    }

    pub(crate) fn viewport_entry_ids(
        &self,
        columns: usize,
        rows: usize,
        prefetch_rows: usize,
    ) -> impl Iterator<Item = EntryId> + '_ {
        let columns = columns.max(1);
        let start_row = self.row_offset.saturating_sub(prefetch_rows);
        let end_row = self.row_offset + rows.max(1) + prefetch_rows;
        let start = start_row.saturating_mul(columns);
        let end = end_row.saturating_mul(columns).min(self.filtered.len());
        self.filtered[start.min(end)..end].iter().copied()
    }

    fn rebuild_filter(&mut self) {
        let selected_id = self.filtered.get(self.selected).copied();
        let old_position = self.selected;
        let needle = self.query.to_lowercase();
        self.filtered.clear();
        self.filtered.extend(
            self.entries
                .iter()
                .filter(|entry| {
                    (needle.is_empty() || entry.search_key.contains(&needle))
                        && (!self.marked_only || self.marked.contains(&entry.id))
                })
                .map(|entry| entry.id),
        );
        self.selected = selected_id
            .and_then(|id| self.filtered.iter().position(|candidate| *candidate == id))
            .unwrap_or_else(|| old_position.min(self.filtered.len().saturating_sub(1)));
        if self.filtered.is_empty() {
            self.row_offset = 0;
        }
    }
}

pub(crate) fn is_supported_image(path: &Path) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| {
            matches!(
                extension.to_ascii_lowercase().as_str(),
                "jpg" | "jpeg" | "png" | "gif" | "webp" | "tif" | "tiff" | "bmp"
            )
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn app_with_images(names: &[&str]) -> App {
        let mut app = App::new("/photos".into());
        for name in names {
            app.add_path(
                PathBuf::from("/photos").join(name),
                SourceRevision::default(),
            );
        }
        app
    }

    #[test]
    fn recognizes_supported_extensions_case_insensitively() {
        for name in [
            "a.jpg", "a.JPEG", "a.png", "a.gif", "a.webp", "a.tiff", "a.bmp",
        ] {
            assert!(is_supported_image(Path::new(name)), "{name}");
        }
        assert!(!is_supported_image(Path::new("notes.txt")));
        assert!(!is_supported_image(Path::new("jpg")));
    }

    #[test]
    fn filters_paths_case_insensitively_and_can_cancel() {
        let mut app = app_with_images(&["Trips/Berlin.JPG", "Family/home.png"]);
        app.start_search();
        for character in "berlin".chars() {
            app.push_search(character);
        }
        assert_eq!(app.filtered, vec![EntryId(0)]);

        app.cancel_search();
        assert_eq!(app.filtered, vec![EntryId(0), EntryId(1)]);
        assert!(app.query.is_empty());
    }

    #[test]
    fn grid_navigation_clamps_and_scrolls() {
        let mut app = app_with_images(&["0.jpg", "1.jpg", "2.jpg", "3.jpg", "4.jpg"]);
        app.move_down(2, 1);
        assert_eq!(app.selected, 2);
        assert_eq!(app.row_offset, 1);

        app.move_down(2, 1);
        assert_eq!(app.selected, 4);
        assert_eq!(app.row_offset, 2);

        app.move_down(2, 1);
        assert_eq!(app.selected, 4);
        app.move_up(2, 1);
        assert_eq!(app.selected, 2);
    }

    #[test]
    fn viewport_prefetch_is_bounded() {
        let names = (0..100)
            .map(|index| format!("{index}.jpg"))
            .collect::<Vec<_>>();
        let borrowed = names.iter().map(String::as_str).collect::<Vec<_>>();
        let mut app = app_with_images(&borrowed);
        app.selected = 50;
        app.ensure_visible(5, 2);

        let visible = app.viewport_entry_ids(5, 2, 1).collect::<Vec<_>>();
        assert!(visible.len() <= 20);
        assert!(visible.contains(&EntryId(50)));
    }

    #[test]
    fn identity_survives_filtering_and_revision_changes() {
        let mut app = app_with_images(&["one.jpg", "two.jpg"]);
        let id = app.filtered[1];
        app.selected = 1;
        app.start_search();
        app.push_search('t');
        assert_eq!(app.selected_entry().unwrap().id, id);

        let changed = SourceRevision {
            size: 42,
            modified_nanoseconds: 9,
        };
        assert_eq!(app.add_path(PathBuf::from("/photos/two.jpg"), changed), id);
        assert_eq!(app.entry(id).unwrap().revision, changed);
        assert_eq!(app.entries.len(), 2);
    }

    #[test]
    fn focus_and_marks_remain_independent_across_filters() {
        let mut app = app_with_images(&["one.jpg", "two.jpg", "three.jpg"]);
        app.selected = 1;
        app.toggle_mark();
        let marked = EntryId(1);
        assert!(app.is_marked(marked));
        app.move_right(3, 1);
        assert_eq!(app.selected_entry().unwrap().id, EntryId(2));
        assert!(app.is_marked(marked));

        app.start_search();
        for character in "three".chars() {
            app.push_search(character);
        }
        assert_eq!(app.filtered, vec![EntryId(2)]);
        assert!(app.is_marked(marked));
        app.cancel_search();
        assert_eq!(app.marked_paths(), vec![PathBuf::from("/photos/two.jpg")]);
    }

    #[test]
    fn range_marks_in_visible_order_and_selected_only_composes_with_search() {
        let mut app = app_with_images(&["zero.jpg", "one.jpg", "two.jpg", "three.jpg", "four.jpg"]);
        app.selected = 1;
        app.toggle_mark();
        app.selected = 3;
        app.mark_range();
        assert_eq!(app.marked_count(), 3);
        assert_eq!(
            app.marked_paths(),
            ["one.jpg", "two.jpg", "three.jpg"].map(|name| PathBuf::from("/photos").join(name))
        );

        app.toggle_marked_only();
        assert_eq!(app.filtered, vec![EntryId(1), EntryId(2), EntryId(3)]);
        app.start_search();
        for character in "two".chars() {
            app.push_search(character);
        }
        assert_eq!(app.filtered, vec![EntryId(2)]);
        assert_eq!(app.marked_count(), 3);
        app.clear_marks();
        assert!(app.filtered.is_empty());
        app.toggle_marked_only();
        assert_eq!(app.filtered, vec![EntryId(2)]);
    }

    #[test]
    fn unmarking_then_remarking_moves_an_entry_to_the_end() {
        let mut app = app_with_images(&["one.jpg", "two.jpg"]);
        app.toggle_mark();
        app.move_right(2, 1);
        app.toggle_mark();
        app.move_left(2, 1);
        app.toggle_mark();
        app.toggle_mark();
        assert_eq!(
            app.marked_paths(),
            vec![
                PathBuf::from("/photos/two.jpg"),
                PathBuf::from("/photos/one.jpg")
            ]
        );
    }
}
