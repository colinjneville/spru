#[derive(Debug)]
#[derive(serde::Serialize, serde::Deserialize)]
pub enum Variant<MemberInfo> {
    UpdateInfo(MemberInfo),
    SetReady(bool),
}