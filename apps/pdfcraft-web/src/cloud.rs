//! Browser host adapter: the editor retains ownership of its native document format.
use sovereign_cloud_ui::{DocumentState, Editor, Snapshot};

pub struct Adapter<'a>(pub &'a mut pdfcraft_ui_egui::PdfCraftApp);
impl Editor for Adapter<'_> {
    fn set_user_profile(&mut self, profile: Option<sovereign_cloud_ui::UserProfile<'_>>) {
        let previous = self.0.comment_prefs.user_id.is_some();
        self.0.comment_prefs.user_id = profile.map(|p| p.id.to_owned());
        if let Some(profile) = profile {
            if self.0.comment_prefs.author != profile.display_name {
                self.0.comment_prefs.author = profile.display_name.to_owned();
            }
        } else if previous {
            self.0.comment_prefs.author = "PDF user".into();
        }
    }

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

#[cfg(test)]
mod identity_tests {
    use super::*;

    #[test]
    fn account_profile_follows_host_and_cannot_restore_identity() {
        let mut app = pdfcraft_ui_egui::PdfCraftApp::new();
        app.comment_prefs.author = "Same display name".into();
        for id in [None, Some("account-a"), Some("account-a"), Some("account-b"), None] {
            Adapter(&mut app).set_user_profile(id.map(|id| sovereign_cloud_ui::UserProfile { id, display_name: "Subnet name" }));
            assert_eq!(app.comment_prefs.user_id.as_deref(), id);
            if id.is_some() {
                assert_eq!(app.comment_prefs.author, "Subnet name");
            }
        }
        Adapter(&mut app).set_user_profile(Some(sovereign_cloud_ui::UserProfile { id: "account-a", display_name: "Subnet name" }));
    }
}
