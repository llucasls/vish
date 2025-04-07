pub fn get_home(name: String) -> Option<String> {
    crate::app::Home::from_username(name)
}
