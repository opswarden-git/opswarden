use std::sync::Arc;

use uuid::Uuid;

use crate::domain::conversation::MessageAttachment;
use crate::domain::error::DomainError;
use crate::ports::TimelineRepo;

pub struct GetTimelineAttachmentUseCase {
    timeline: Arc<dyn TimelineRepo>,
}

impl GetTimelineAttachmentUseCase {
    pub fn new(timeline: Arc<dyn TimelineRepo>) -> Self {
        Self { timeline }
    }

    pub async fn get(
        &self,
        attachment_id: Uuid,
        requester_id: Uuid,
    ) -> Result<MessageAttachment, DomainError> {
        self.timeline
            .find_attachment_for_member(attachment_id, requester_id)
            .await?
            .ok_or(DomainError::Forbidden)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::incident::tests::MockTimelineRepo;
    use crate::domain::timeline::TimelineEntry;

    #[tokio::test]
    async fn member_downloads_a_bounded_incident_attachment() {
        let timeline = Arc::new(MockTimelineRepo::default());
        let entry = TimelineEntry::new_with_attachments(
            Uuid::new_v4(),
            Uuid::new_v4(),
            "",
            vec![("runbook.txt".into(), "text/plain".into(), b"steps".to_vec())],
        )
        .unwrap();
        let attachment_id = entry.attachments[0].id;
        timeline.append_entry(&entry).await.unwrap();
        let member_id = Uuid::new_v4();
        timeline.allow_attachment_member(member_id);

        let use_case = GetTimelineAttachmentUseCase::new(timeline);
        let attachment = use_case.get(attachment_id, member_id).await.unwrap();

        assert_eq!(attachment.file_name, "runbook.txt");
        assert_eq!(attachment.content, b"steps");
        assert_eq!(
            use_case.get(attachment_id, Uuid::new_v4()).await,
            Err(DomainError::Forbidden)
        );
    }
}
