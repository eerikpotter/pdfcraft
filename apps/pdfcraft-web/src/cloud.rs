//! Browser host adapter: the editor retains ownership of its native document format.
use sovereign_cloud_ui::{DocumentState, Editor, Snapshot};

pub struct Adapter<'a>(pub &'a mut pdfcraft_ui_egui::PdfCraftApp);
impl Editor for Adapter<'_> {
    fn document(&self) -> Option<DocumentState> {
        let (_, id) = self.0.active_ids()?;
        let d = self.0.session.get(id)?;
        Some(DocumentState { id: id.0, revision: d.edit_generation(), name: d.name.clone(), dirty: d.dirty })
    }
    fn capture(&self) -> Result<Snapshot, String> {
        let document = self.document().ok_or("No PDF is open")?;
        let bytes = self.0.session.save_bytes(pdfcraft_engine::DocId(document.id)).map_err(|e| e.to_string())?;
        Ok(Snapshot { document, encode: Box::new(move || Ok(bytes.as_ref().clone())) })
    }
    fn saved(&mut self, id: u64, revision: u64, bytes: &[u8]) -> Result<Option<u64>, String> {
        let id = pdfcraft_engine::DocId(id);
        if self.0.session.get(id).is_some_and(|d| d.edit_generation() == revision) {
            self.0.session.mark_saved(id, std::sync::Arc::new(bytes.to_vec()), None).map_err(|e| e.to_string())?;
            return Ok(self.0.session.get(id).map(|d| d.edit_generation()));
        }
        Ok(None)
    }
    fn open(&mut self, name: &str, bytes: &[u8]) -> Result<DocumentState, String> {
        let previous = self.0.active_ids().map(|(_, id)| id);
        self.0.open_bytes(name, None, bytes.to_vec())?;
        if self.0.active_ids().map(|(_, id)| id) == previous {
            return Err("This PDF needs its password in the editor. After unlocking, use Save As for its next cloud save.".into());
        }
        self.document().ok_or_else(|| "Could not open the PDF".into())
    }
}
