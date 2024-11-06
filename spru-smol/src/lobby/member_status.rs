use crate::{lobby::ReadyStatus, router};

#[derive(Debug)]
pub struct MemberStatus<MemberInfo> {
    pub(crate) member_info: MemberInfo,
    pub(crate) ready_status: ReadyStatus,
}

impl<MemberInfo> MemberStatus<MemberInfo> {
    pub fn member_info(&self) -> &MemberInfo {
        &self.member_info
    }

    pub fn is_ready(&self) -> bool {
        self.ready_status.is_ready()
    }
}


