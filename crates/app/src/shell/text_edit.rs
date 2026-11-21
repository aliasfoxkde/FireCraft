/// In-canvas text editing (20.1): the live editor state for one text
/// object. `buffer` is what the overlay `TextEdit` shows; commit sends
/// it through `text.setText` when it differs from `original`; cancel
/// (Esc, or switching tools) drops it without touching the document.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct TextEditState {
    /// The text object being edited.
    pub(crate) id: doc::ObjectId,
    /// The characters before editing started.
    pub(crate) original: String,
    /// The live edit buffer.
    pub(crate) buffer: String,
}
