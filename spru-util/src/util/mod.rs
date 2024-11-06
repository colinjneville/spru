pub enum ChangeType<T> {
    Create(T),
    Update(T),
    Destroy,
}

pub struct Change<Id, T> {
    pub id: Id,
    pub change_type: ChangeType<T>,
}