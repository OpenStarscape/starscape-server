pub trait Object {
    fn process_messages(&self, messages: &str);
    fn get_updates(&self) -> String;
}
