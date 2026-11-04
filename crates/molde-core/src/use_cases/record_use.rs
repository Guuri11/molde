use super::*;

impl Engine {
    pub fn record_use(&mut self, id: SnippetId, project_root: String) -> Result<(), String> {
        let at = self.ports.clock.now();
        self.ports.usage.record(&UsageEvent {
            id,
            project_root,
            at,
        })
    }
}
