pub fn get_home(name: String) -> Option<String> {
    crate::util::Home::from_username(name)
}
