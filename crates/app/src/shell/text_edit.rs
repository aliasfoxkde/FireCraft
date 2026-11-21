
use super::*;

impl Shell {
    /// Begin in-canvas editing of the text object under `id` (20.1):
    /// snapshot its characters and open the overlay. Re-opening on the
    /// same object keeps the live buffer (a click back into the edit
    /// must not wipe what was typed).
    pub(crate) fn begin_text_edit(&mut self, id: doc::ObjectId) {
        if self.text_edit.as_ref().is_some_and(|edit| edit.id == id) {
            return;
        }
        let original = match self.editor.document().registry.get(id) {
            Some(doc::ObjectData::Text(text)) => text
                .runs
                .iter()
                .map(|run| run.text.as_str())
                .collect::<String>(),
            _ => return,
        };
        self.text_edit = Some(TextEditState {
            id,
            buffer: original.clone(),
            original,
        });
    }

    /// Commit the text-edit overlay: `text.setText` with the buffer
    /// when it changed; then the session closes the editor either way.
    pub(crate) fn commit_text_edit(&mut self) {
        let Some(edit) = self.text_edit.take() else {
            return;
        };
        if edit.buffer == edit.original {
            return;
        }
        if !self.run_command(
            "text.setText",
            serde_json::json!({ "id": edit.id, "text": edit.buffer }),
        ) {
            self.status = "text edit failed".to_owned();
        }
    }

    /// Cancel the text-edit overlay without touching the document.
    pub(crate) fn cancel_text_edit(&mut self) {
        self.text_edit = None;
    }

    /// The Text Editor / Edit Image Object row (fwref `cmd_TextEditor`,
    /// `cmd_EditImageObject`): a selected text object opens the
    /// in-canvas text overlay; anything else reports that the kind has
    /// no editor in this build.
    pub(crate) fn edit_text_selection(&mut self) {
        let Some(id) = self.editor.selection else {
            self.status = "select a text object to edit it".to_owned();
            return;
        };
        let is_text = matches!(
            self.editor.session.document().registry.get(id),
            Some(doc::ObjectData::Text(_))
        );
        if is_text {
            self.begin_text_edit(id);
            self.status = "editing text".to_owned();
        } else {
            self.status = "the selected kind has no editor in this build".to_owned();
        }
    }
}
