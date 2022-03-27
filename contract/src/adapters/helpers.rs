use cosmwasm_std::Event;

/// Returns `true` if the event contains the specified key-value pair
pub fn event_contains_attr(event: &Event, key: &str, value: &str) -> bool {
    event
        .attributes
        .iter()
        .any(|attr| attr.key == key && attr.value == value)
}
